use anchor_lang::{
    solana_program::{instruction::Instruction, pubkey::Pubkey},
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::spl_token_2022::{
        extension::{
            permanent_delegate::PermanentDelegate, transfer_hook::TransferHook,
            BaseStateWithExtensions, StateWithExtensions,
        },
        state::{Account as SplTokenAccount, Mint as SplMint},
    },
};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use spl_tlv_account_resolution::solana_instruction;
use whitelist_transfer_hook::{accounts as hook_accounts, instruction as hook_instruction};
use whitelist_transfer_vault::{
    accounts, instruction, state::WhitelistEntry, MINT_SEED, VAULT_CONFIG_SEED, WHITELIST_SEED,
};

const TOKEN_2022_ID: Pubkey = anchor_spl::token_interface::spl_token_2022::ID;
const ASSOCIATED_TOKEN_ID: Pubkey = anchor_spl::associated_token::ID;
const SYSTEM_ID: Pubkey = anchor_lang::system_program::ID;

fn setup_svm() -> (LiteSVM, Pubkey, Pubkey) {
    let pid = whitelist_transfer_vault::id();
    let hook_pid = whitelist_transfer_hook::id();
    let mut svm = LiteSVM::new().with_default_programs();
    let prog_bytes = include_bytes!("../../../target/deploy/whitelist_transfer_vault.so");
    svm.add_program(pid, prog_bytes).unwrap();
    let hook_prog_bytes = include_bytes!("../../../target/deploy/whitelist_transfer_hook.so");
    svm.add_program(hook_pid, hook_prog_bytes).unwrap();
    (svm, pid, hook_pid)
}

fn fund_accounts(svm: &mut LiteSVM, keys: &[&Pubkey]) {
    for k in keys {
        svm.airdrop(k, 10_000_000_000).unwrap();
    }
}

fn derive_vault(pid: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[VAULT_CONFIG_SEED], pid).0
}

fn derive_mint(pid: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[MINT_SEED], pid).0
}

fn derive_extra_meta(mint: &Pubkey, hook_pid: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"extra-account-metas", mint.as_ref()], hook_pid).0
}

fn derive_ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(owner, mint, &TOKEN_2022_ID)
}

fn derive_whitelist_pda(user: &Pubkey, pid: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[WHITELIST_SEED, user.as_ref()], pid).0
}

fn submit_tx(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instructions: &[Instruction],
    signers: &[&Keypair],
) -> bool {
    let blockhash = svm.latest_blockhash();
    let mut all_signers = vec![payer];
    for s in signers {
        if s.pubkey() != payer.pubkey() {
            all_signers.push(s);
        }
    }
    let msg = Message::new_with_blockhash(instructions, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &all_signers).unwrap();
    let outcome = svm.send_transaction(tx);
    if let Err(e) = &outcome {
        eprintln!("transaction failed: {e:?}");
    }
    outcome.is_ok()
}

fn ix_initialize(
    pid: Pubkey,
    hook_pid: Pubkey,
    admin: Pubkey,
) -> (Instruction, Instruction) {
    let vault = derive_vault(&pid);
    let mint = derive_mint(&pid);
    let vault_token_acc = derive_ata(&vault, &mint);
    let extra_meta = derive_extra_meta(&mint, &hook_pid);

    let vault_ix = Instruction::new_with_bytes(
        pid,
        &instruction::Initialize {}.data(),
        accounts::Initialize {
            admin,
            vault_pda: vault,
            mint,
            vault_token_account: vault_token_acc,
            vault_whitelist_entry: derive_whitelist_pda(&vault, &pid),
            associated_token_program: ASSOCIATED_TOKEN_ID,
            token_program: TOKEN_2022_ID,
            system_program: SYSTEM_ID,
        }
        .to_account_metas(None),
    );

    let hook_ix = Instruction::new_with_bytes(
        hook_pid,
        &hook_instruction::InitializeExtraAccountMetas {}.data(),
        hook_accounts::InitializeExtraAccountMetaList {
            payer: admin,
            mint,
            extra_account_meta_list: extra_meta,
            system_program: SYSTEM_ID,
        }
        .to_account_metas(None),
    );

    (vault_ix, hook_ix)
}

fn ix_add_to_whitelist(pid: Pubkey, admin: Pubkey, user: Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        pid,
        &instruction::AddToWhitelist { user }.data(),
        accounts::AddToWhitelist {
            admin,
            vault_pda: derive_vault(&pid),
            whitelist_entry: derive_whitelist_pda(&user, &pid),
            system_program: SYSTEM_ID,
        }
        .to_account_metas(None),
    )
}

fn ix_remove_from_whitelist(pid: Pubkey, admin: Pubkey, user: Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        pid,
        &instruction::RemoveFromWhitelist { user }.data(),
        accounts::RemoveFromWhitelist {
            admin,
            vault_pda: derive_vault(&pid),
            whitelist_entry: derive_whitelist_pda(&user, &pid),
        }
        .to_account_metas(None),
    )
}

fn ix_mint_to_user(pid: Pubkey, admin: Pubkey, recipient: Pubkey, amount: u64) -> Instruction {
    let mint = derive_mint(&pid);
    Instruction::new_with_bytes(
        pid,
        &instruction::MintToken { amount }.data(),
        accounts::MintToken {
            admin,
            user: recipient,
            vault_pda: derive_vault(&pid),
            mint,
            user_token_account: derive_ata(&recipient, &mint),
            associated_token_program: ASSOCIATED_TOKEN_ID,
            token_program: TOKEN_2022_ID,
            system_program: SYSTEM_ID,
        }
        .to_account_metas(None),
    )
}

fn ix_deposit(
    pid: Pubkey,
    hook_pid: Pubkey,
    user: Pubkey,
    amount: u64,
) -> Instruction {
    let vault = derive_vault(&pid);
    let mint = derive_mint(&pid);
    let extra_meta = derive_extra_meta(&mint, &hook_pid);
    Instruction::new_with_bytes(
        pid,
        &instruction::Deposit { amount }.data(),
        accounts::Deposit {
            user,
            vault_pda: vault,
            user_token_account: derive_ata(&user, &mint),
            vault_token_account: derive_ata(&vault, &mint),
            mint,
            user_whitelist_entry: derive_whitelist_pda(&user, &pid),
            vault_whitelist_entry: derive_whitelist_pda(&vault, &pid),
            extra_account_meta_list: extra_meta,
            vault_program: pid,
            hook_program: hook_pid,
            token_program: TOKEN_2022_ID,
        }
        .to_account_metas(None),
    )
}

fn ix_withdraw(
    pid: Pubkey,
    hook_pid: Pubkey,
    user: Pubkey,
    amount: u64,
) -> Instruction {
    let vault = derive_vault(&pid);
    let mint = derive_mint(&pid);
    let extra_meta = derive_extra_meta(&mint, &hook_pid);
    Instruction::new_with_bytes(
        pid,
        &instruction::Withdraw { amount }.data(),
        accounts::Withdraw {
            user,
            vault_pda: vault,
            vault_token_account: derive_ata(&vault, &mint),
            user_token_account: derive_ata(&user, &mint),
            mint,
            user_whitelist_entry: derive_whitelist_pda(&user, &pid),
            vault_whitelist_entry: derive_whitelist_pda(&vault, &pid),
            extra_account_meta_list: extra_meta,
            vault_program: pid,
            hook_program: hook_pid,
            token_program: TOKEN_2022_ID,
        }
        .to_account_metas(None),
    )
}

fn ix_raw_transfer(
    pid: Pubkey,
    hook_pid: Pubkey,
    owner: Pubkey,
    dest_owner: Pubkey,
    amount: u64,
) -> Instruction {
    let mint = derive_mint(&pid);
    let extra_meta = derive_extra_meta(&mint, &hook_pid);
    let src_ata = derive_ata(&owner, &mint);
    let dst_ata = derive_ata(&dest_owner, &mint);

    let mut ix = anchor_spl::token_interface::spl_token_2022::instruction::transfer_checked(
        &TOKEN_2022_ID,
        &src_ata,
        &mint,
        &dst_ata,
        &owner,
        &[],
        amount,
        6,
    )
    .unwrap();

    ix.accounts
        .push(solana_instruction::AccountMeta::new_readonly(pid, false));
    ix.accounts
        .push(solana_instruction::AccountMeta::new_readonly(
            derive_whitelist_pda(&owner, &pid),
            false,
        ));
    ix.accounts
        .push(solana_instruction::AccountMeta::new_readonly(
            derive_whitelist_pda(&dest_owner, &pid),
            false,
        ));
    ix.accounts
        .push(solana_instruction::AccountMeta::new_readonly(extra_meta, false));
    ix.accounts
        .push(solana_instruction::AccountMeta::new_readonly(hook_pid, false));
    ix
}

fn read_token_balance(svm: &LiteSVM, account: Pubkey) -> u64 {
    let acc = svm.get_account(&account).unwrap();
    StateWithExtensions::<SplTokenAccount>::unpack(&acc.data)
        .unwrap()
        .base
        .amount
}

fn read_whitelist_balance(svm: &LiteSVM, user: Pubkey, pid: &Pubkey) -> u64 {
    let pda = derive_whitelist_pda(&user, pid);
    let acc = svm.get_account(&pda).unwrap();
    WhitelistEntry::try_deserialize(&mut acc.data.as_slice())
        .unwrap()
        .balance
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[test]
fn vault_setup_initializes_mint_with_correct_extensions() {
    let (mut svm, pid, hook_pid) = setup_svm();
    let admin = Keypair::new();
    fund_accounts(&mut svm, &[&admin.pubkey()]);

    let (vault_ix, hook_ix) = ix_initialize(pid, hook_pid, admin.pubkey());
    assert!(submit_tx(&mut svm, &admin, &[vault_ix, hook_ix], &[]));

    let vault = derive_vault(&pid);
    let mint = derive_mint(&pid);
    let vault_token_acc = derive_ata(&vault, &mint);
    let extra_meta = derive_extra_meta(&mint, &hook_pid);

    assert!(svm.get_account(&vault).is_some());
    assert!(svm.get_account(&vault_token_acc).is_some());
    assert!(svm.get_account(&extra_meta).is_some());

    let mint_acc = svm.get_account(&mint).unwrap();
    let mint_state = StateWithExtensions::<SplMint>::unpack(&mint_acc.data).unwrap();
    assert!(mint_state.get_extension::<TransferHook>().is_ok());
    assert!(mint_state.get_extension::<PermanentDelegate>().is_ok());
}

#[test]
fn approved_users_complete_full_deposit_withdraw_transfer_flow() {
    println!(
        "whitelist_transfer_hook::id() = {}",
        whitelist_transfer_hook::id()
    );
    println!(
        "hook_program_id used in test = {}",
        whitelist_transfer_hook::ID
    );
    println!(
        "expected TRANSFER_HOOK_PROGRAM_ID = {}",
        whitelist_transfer_vault::TRANSFER_HOOK_PROGRAM_ID
    );
    let (mut svm, pid, hook_pid) = setup_svm();
    let admin = Keypair::new();
    let alice = Keypair::new();
    let bob = Keypair::new();
    fund_accounts(&mut svm, &[&admin.pubkey(), &alice.pubkey(), &bob.pubkey()]);

    let (vault_ix, hook_ix) = ix_initialize(pid, hook_pid, admin.pubkey());
    assert!(submit_tx(&mut svm, &admin, &[vault_ix, hook_ix], &[]));

    assert!(submit_tx(
        &mut svm,
        &admin,
        &[ix_add_to_whitelist(pid, admin.pubkey(), alice.pubkey())],
        &[]
    ));
    assert!(submit_tx(
        &mut svm,
        &admin,
        &[ix_add_to_whitelist(pid, admin.pubkey(), bob.pubkey())],
        &[]
    ));
    assert!(submit_tx(
        &mut svm,
        &admin,
        &[ix_mint_to_user(pid, admin.pubkey(), alice.pubkey(), 1_000)],
        &[]
    ));
    assert!(submit_tx(
        &mut svm,
        &admin,
        &[ix_mint_to_user(pid, admin.pubkey(), bob.pubkey(), 0)],
        &[]
    ));

    let vault = derive_vault(&pid);
    let mint = derive_mint(&pid);
    let vault_token_acc = derive_ata(&vault, &mint);
    let bob_token_acc = derive_ata(&bob.pubkey(), &mint);

    assert!(submit_tx(
        &mut svm,
        &alice,
        &[ix_deposit(pid, hook_pid, alice.pubkey(), 400)],
        &[]
    ));
    assert_eq!(read_token_balance(&svm, vault_token_acc), 400);
    assert_eq!(read_whitelist_balance(&svm, alice.pubkey(), &pid), 400);

    assert!(submit_tx(
        &mut svm,
        &alice,
        &[ix_withdraw(pid, hook_pid, alice.pubkey(), 150)],
        &[]
    ));
    assert_eq!(read_token_balance(&svm, vault_token_acc), 250);
    assert_eq!(read_whitelist_balance(&svm, alice.pubkey(), &pid), 250);

    assert!(submit_tx(
        &mut svm,
        &alice,
        &[ix_raw_transfer(pid, hook_pid, alice.pubkey(), bob.pubkey(), 100)],
        &[]
    ));
    assert_eq!(read_token_balance(&svm, bob_token_acc), 100);
}