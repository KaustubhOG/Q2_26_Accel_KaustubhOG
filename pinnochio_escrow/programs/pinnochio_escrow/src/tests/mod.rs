#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use litesvm::LiteSVM;
    use litesvm_token::{
        CreateAssociatedTokenAccount, CreateMint, MintTo,
        spl_token::{self},
    };
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
    const ESCROW_LEN: usize = 121;

    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn so_path() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
        for subdir in &["sbpf-solana-solana", "sbf-solana-solana"] {
            let p = workspace_root
                .join("target")
                .join(subdir)
                .join("release/accel_p_escrow.so");
            if p.exists() {
                return p;
            }
        }
        workspace_root.join("target/deploy/accel_p_escrow.so")
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let program_data = std::fs::read(so_path())
            .expect("Failed to read escrow.so -- run `cargo build-sbf` first");
        svm.add_program(program_id(), &program_data)
            .expect("Failed to add program");

        (svm, payer)
    }

    fn escrow_pda(maker: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"escrow", maker.as_ref()], &program_id())
    }

    fn ata_program() -> Pubkey {
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
            .parse()
            .unwrap()
    }

    fn system_program() -> Pubkey {
        solana_sdk_ids::system_program::ID
    }

    fn read_token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
        let account = svm.get_account(ata).expect("token account not found");
        let bytes: [u8; 8] = account.data[64..72].try_into().unwrap();
        u64::from_le_bytes(bytes)
    }

    struct EscrowSetup {
        svm: LiteSVM,
        maker: Keypair,
        mint_a: Pubkey,
        mint_b: Pubkey,
        maker_ata_a: Pubkey,
        escrow: Pubkey,
        _escrow_bump: u8,
        vault: Pubkey,
        amount_to_receive: u64,
        amount_to_give: u64,
    }

    fn setup_make(
        amount_to_receive: u64,
        amount_to_give: u64,
        mint_amount: u64,
        expiry_duration: i64,
    ) -> EscrowSetup {
        let (mut svm, maker) = setup();

        let mint_a = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, mint_amount)
            .send()
            .unwrap();

        let (escrow, escrow_bump) = escrow_pda(&maker.pubkey());
        let vault = spl_associated_token_account::get_associated_token_address(&escrow, &mint_a);

        let make_data = [
            vec![0u8],
            vec![escrow_bump],
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
            expiry_duration.to_le_bytes().to_vec(),
        ]
        .concat();

        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(ata_program(), false),
            ],
            data: make_data,
        };

        let msg = Message::new(&[ix], Some(&maker.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[&maker], msg, blockhash);
        svm.send_transaction(tx).expect("Make instruction failed");

        EscrowSetup {
            svm,
            maker,
            mint_a,
            mint_b,
            maker_ata_a,
            escrow,
            _escrow_bump: escrow_bump,
            vault,
            amount_to_receive,
            amount_to_give,
        }
    }

    struct TakeSetup {
        svm: LiteSVM,
        maker: Keypair,
        taker: Keypair,
        mint_a: Pubkey,
        mint_b: Pubkey,
        maker_ata_a: Pubkey,
        maker_ata_b: Pubkey,
        taker_ata_a: Pubkey,
        taker_ata_b: Pubkey,
        escrow: Pubkey,
        vault: Pubkey,
        amount_to_receive: u64,
        amount_to_give: u64,
    }

    fn setup_take(
        amount_to_receive: u64,
        amount_to_give: u64,
        maker_mint_amount: u64,
        taker_mint_amount: u64,
    ) -> TakeSetup {
        let escrow_setup = setup_make(amount_to_receive, amount_to_give, maker_mint_amount, 3600);

        let (mut svm, maker) = (escrow_setup.svm, escrow_setup.maker);
        let taker = Keypair::new();
        svm.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &taker, &escrow_setup.mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &taker, &escrow_setup.mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // maker is the mint_b authority, not taker
        MintTo::new(
            &mut svm,
            &maker,
            &escrow_setup.mint_b,
            &taker_ata_b,
            taker_mint_amount,
        )
        .send()
        .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &escrow_setup.mint_b)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let take_data: Vec<u8> = vec![1u8];

        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(taker.pubkey(), true),
                AccountMeta::new(maker.pubkey(), false),
                AccountMeta::new_readonly(escrow_setup.mint_a, false),
                AccountMeta::new_readonly(escrow_setup.mint_b, false),
                AccountMeta::new(escrow_setup.escrow, false),
                AccountMeta::new(taker_ata_a, false),
                AccountMeta::new(taker_ata_b, false),
                AccountMeta::new(maker_ata_b, false),
                AccountMeta::new(escrow_setup.vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            ],
            data: take_data,
        };

        let msg = Message::new(&[ix], Some(&taker.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[&taker], msg, blockhash);
        svm.send_transaction(tx).expect("Take instruction failed");

        TakeSetup {
            svm,
            maker,
            taker,
            mint_a: escrow_setup.mint_a,
            mint_b: escrow_setup.mint_b,
            maker_ata_a: escrow_setup.maker_ata_a,
            maker_ata_b,
            taker_ata_a,
            taker_ata_b,
            escrow: escrow_setup.escrow,
            vault: escrow_setup.vault,
            amount_to_receive,
            amount_to_give,
        }
    }

    struct RefundSetup {
        svm: LiteSVM,
        maker: Keypair,
        mint_a: Pubkey,
        maker_ata_a: Pubkey,
        escrow: Pubkey,
        vault: Pubkey,
        amount_to_receive: u64,
        amount_to_give: u64,
    }

    fn setup_refund(
        amount_to_receive: u64,
        amount_to_give: u64,
        maker_mint_amount: u64,
    ) -> RefundSetup {
        let mut escrow_setup =
            setup_make(amount_to_receive, amount_to_give, maker_mint_amount, 3600);

        let refund_data: Vec<u8> = vec![2u8];

        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(escrow_setup.maker.pubkey(), true),
                AccountMeta::new_readonly(escrow_setup.mint_a, false),
                AccountMeta::new(escrow_setup.escrow, false),
                AccountMeta::new(escrow_setup.maker_ata_a, false),
                AccountMeta::new(escrow_setup.vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            ],
            data: refund_data,
        };

        let msg = Message::new(&[ix], Some(&escrow_setup.maker.pubkey()));
        let blockhash = escrow_setup.svm.latest_blockhash();
        let tx = Transaction::new(&[&escrow_setup.maker], msg, blockhash);
        escrow_setup
            .svm
            .send_transaction(tx)
            .expect("Refund instruction failed");

        RefundSetup {
            svm: escrow_setup.svm,
            maker: escrow_setup.maker,
            mint_a: escrow_setup.mint_a,
            maker_ata_a: escrow_setup.maker_ata_a,
            escrow: escrow_setup.escrow,
            vault: escrow_setup.vault,
            amount_to_receive,
            amount_to_give,
        }
    }

    #[test]
    fn test_make() {
        let s = setup_make(100_000_000, 500_000_000, 1_000_000_000, 3600);

        let escrow_account = s.svm.get_account(&s.escrow).expect("escrow not found");
        assert_eq!(escrow_account.owner, program_id());
        assert_eq!(escrow_account.data.len(), ESCROW_LEN);

        let vault_balance = read_token_balance(&s.svm, &s.vault);
        assert_eq!(vault_balance, s.amount_to_give);

        let maker_balance = read_token_balance(&s.svm, &s.maker_ata_a);
        assert_eq!(maker_balance, 1_000_000_000 - s.amount_to_give);
    }

    #[test]
    fn test_make_escrow_state_stored_correctly() {
        let s = setup_make(200_000_000, 300_000_000, 1_000_000_000, 7200);

        let escrow_account = s.svm.get_account(&s.escrow).expect("escrow not found");
        let data = &escrow_account.data;

        let stored_maker = Pubkey::try_from(&data[0..32]).unwrap();
        assert_eq!(stored_maker, s.maker.pubkey());

        let stored_mint_a = Pubkey::try_from(&data[32..64]).unwrap();
        assert_eq!(stored_mint_a, s.mint_a);

        let stored_mint_b = Pubkey::try_from(&data[64..96]).unwrap();
        assert_eq!(stored_mint_b, s.mint_b);

        let stored_receive = u64::from_le_bytes(data[96..104].try_into().unwrap());
        assert_eq!(stored_receive, 200_000_000);

        let stored_give = u64::from_le_bytes(data[104..112].try_into().unwrap());
        assert_eq!(stored_give, 300_000_000);

        let stored_timestamp = i64::from_le_bytes(data[112..120].try_into().unwrap());
        assert!(stored_timestamp > 0);

        let stored_bump = data[120];
        assert!(stored_bump > 0);
    }

    #[test]
    fn test_make_zero_amount_fails() {
        let (mut svm, maker) = setup();

        let mint_a = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 1_000_000_000)
            .send()
            .unwrap();

        let (escrow, escrow_bump) = escrow_pda(&maker.pubkey());
        let vault = spl_associated_token_account::get_associated_token_address(&escrow, &mint_a);

        let make_data = [
            vec![0u8],
            vec![escrow_bump],
            0u64.to_le_bytes().to_vec(),
            500_000_000u64.to_le_bytes().to_vec(),
            3600i64.to_le_bytes().to_vec(),
        ]
        .concat();

        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(ata_program(), false),
            ],
            data: make_data,
        };

        let msg = Message::new(&[ix], Some(&maker.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[&maker], msg, blockhash);
        let err = svm.send_transaction(tx);

        assert!(err.is_err(), "Make should fail with zero amount_to_receive");
    }

    #[test]
    fn test_take() {
        let s = setup_take(100_000_000, 500_000_000, 1_000_000_000, 2_000_000_000);

        assert!(s.svm.get_account(&s.escrow).is_none());
        assert!(s.svm.get_account(&s.vault).is_none());

        let taker_a_balance = read_token_balance(&s.svm, &s.taker_ata_a);
        assert_eq!(taker_a_balance, s.amount_to_give);

        let maker_b_balance = read_token_balance(&s.svm, &s.maker_ata_b);
        assert_eq!(maker_b_balance, s.amount_to_receive);

        let taker_b_balance = read_token_balance(&s.svm, &s.taker_ata_b);
        assert_eq!(taker_b_balance, 2_000_000_000 - s.amount_to_receive);

        let maker_a_balance = read_token_balance(&s.svm, &s.maker_ata_a);
        assert_eq!(maker_a_balance, 1_000_000_000 - s.amount_to_give);
    }

    #[test]
    fn test_take_insufficient_taker_balance() {
        let mut escrow_setup = setup_make(100_000_000, 500_000_000, 1_000_000_000, 3600);

        let taker = Keypair::new();
        escrow_setup
            .svm
            .airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let taker_ata_a =
            CreateAssociatedTokenAccount::new(&mut escrow_setup.svm, &taker, &escrow_setup.mint_a)
                .owner(&taker.pubkey())
                .send()
                .unwrap();

        let taker_ata_b =
            CreateAssociatedTokenAccount::new(&mut escrow_setup.svm, &taker, &escrow_setup.mint_b)
                .owner(&taker.pubkey())
                .send()
                .unwrap();

        // maker is the mint_b authority
        MintTo::new(
            &mut escrow_setup.svm,
            &escrow_setup.maker,
            &escrow_setup.mint_b,
            &taker_ata_b,
            50_000_000,
        )
        .send()
        .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(
            &mut escrow_setup.svm,
            &escrow_setup.maker,
            &escrow_setup.mint_b,
        )
        .owner(&escrow_setup.maker.pubkey())
        .send()
        .unwrap();

        let take_data: Vec<u8> = vec![1u8];
        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(taker.pubkey(), true),
                AccountMeta::new(escrow_setup.maker.pubkey(), false),
                AccountMeta::new_readonly(escrow_setup.mint_a, false),
                AccountMeta::new_readonly(escrow_setup.mint_b, false),
                AccountMeta::new(escrow_setup.escrow, false),
                AccountMeta::new(taker_ata_a, false),
                AccountMeta::new(taker_ata_b, false),
                AccountMeta::new(maker_ata_b, false),
                AccountMeta::new(escrow_setup.vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            ],
            data: take_data,
        };

        let msg = Message::new(&[ix], Some(&taker.pubkey()));
        let blockhash = escrow_setup.svm.latest_blockhash();
        let tx = Transaction::new(&[&taker], msg, blockhash);
        let err = escrow_setup.svm.send_transaction(tx);

        assert!(
            err.is_err(),
            "Take should fail with insufficient taker balance"
        );
        assert!(escrow_setup.svm.get_account(&escrow_setup.escrow).is_some());
    }

    #[test]
    fn test_take_escrow_cleaned_up() {
        let s = setup_take(100_000_000, 500_000_000, 1_000_000_000, 2_000_000_000);

        assert!(s.svm.get_account(&s.escrow).is_none());
        assert!(s.svm.get_account(&s.vault).is_none());
    }

    #[test]
    fn test_take_permissionless_any_signer_can_take() {
        // escrow is permissionless: any signer with enough token_b can take
        let mut escrow_setup = setup_make(100_000_000, 500_000_000, 1_000_000_000, 3600);

        let third_party = Keypair::new();
        escrow_setup
            .svm
            .airdrop(&third_party.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let third_party_ata_a = CreateAssociatedTokenAccount::new(
            &mut escrow_setup.svm,
            &third_party,
            &escrow_setup.mint_a,
        )
        .owner(&third_party.pubkey())
        .send()
        .unwrap();

        let third_party_ata_b = CreateAssociatedTokenAccount::new(
            &mut escrow_setup.svm,
            &third_party,
            &escrow_setup.mint_b,
        )
        .owner(&third_party.pubkey())
        .send()
        .unwrap();

        // maker is the mint_b authority
        MintTo::new(
            &mut escrow_setup.svm,
            &escrow_setup.maker,
            &escrow_setup.mint_b,
            &third_party_ata_b,
            2_000_000_000,
        )
        .send()
        .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(
            &mut escrow_setup.svm,
            &escrow_setup.maker,
            &escrow_setup.mint_b,
        )
        .owner(&escrow_setup.maker.pubkey())
        .send()
        .unwrap();

        let take_data: Vec<u8> = vec![1u8];
        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(third_party.pubkey(), true),
                AccountMeta::new(escrow_setup.maker.pubkey(), false),
                AccountMeta::new_readonly(escrow_setup.mint_a, false),
                AccountMeta::new_readonly(escrow_setup.mint_b, false),
                AccountMeta::new(escrow_setup.escrow, false),
                AccountMeta::new(third_party_ata_a, false),
                AccountMeta::new(third_party_ata_b, false),
                AccountMeta::new(maker_ata_b, false),
                AccountMeta::new(escrow_setup.vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            ],
            data: take_data,
        };

        let msg = Message::new(&[ix], Some(&third_party.pubkey()));
        let blockhash = escrow_setup.svm.latest_blockhash();
        let tx = Transaction::new(&[&third_party], msg, blockhash);
        let result = escrow_setup.svm.send_transaction(tx);

        // permissionless: any valid signer with enough token_b can take
        assert!(
            result.is_ok(),
            "third party with valid funds must be able to take"
        );
        assert!(escrow_setup.svm.get_account(&escrow_setup.escrow).is_none());
        assert!(escrow_setup.svm.get_account(&escrow_setup.vault).is_none());
    }

    #[test]
    fn test_refund() {
        let initial_balance = 1_000_000_000u64;
        let s = setup_refund(100_000_000, 500_000_000, initial_balance);

        let maker_balance = read_token_balance(&s.svm, &s.maker_ata_a);
        assert_eq!(maker_balance, initial_balance);

        assert!(s.svm.get_account(&s.escrow).is_none());
        assert!(s.svm.get_account(&s.vault).is_none());
    }

    #[test]
    fn test_refund_only_maker_can_refund() {
        let mut escrow_setup = setup_make(100_000_000, 500_000_000, 1_000_000_000, 3600);

        let attacker = Keypair::new();
        escrow_setup
            .svm
            .airdrop(&attacker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        let attacker_ata_a = CreateAssociatedTokenAccount::new(
            &mut escrow_setup.svm,
            &attacker,
            &escrow_setup.mint_a,
        )
        .owner(&attacker.pubkey())
        .send()
        .unwrap();

        let refund_data: Vec<u8> = vec![2u8];
        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(attacker.pubkey(), true),
                AccountMeta::new_readonly(escrow_setup.mint_a, false),
                AccountMeta::new(escrow_setup.escrow, false),
                AccountMeta::new(attacker_ata_a, false),
                AccountMeta::new(escrow_setup.vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            ],
            data: refund_data,
        };

        let msg = Message::new(&[ix], Some(&attacker.pubkey()));
        let blockhash = escrow_setup.svm.latest_blockhash();
        let tx = Transaction::new(&[&attacker], msg, blockhash);
        let err = escrow_setup.svm.send_transaction(tx);

        assert!(err.is_err(), "Refund should fail when called by non-maker");
        assert!(escrow_setup.svm.get_account(&escrow_setup.escrow).is_some());
    }

    #[test]
    fn test_refund_escrow_cleaned_up() {
        let s = setup_refund(100_000_000, 500_000_000, 1_000_000_000);

        assert!(s.svm.get_account(&s.escrow).is_none());
        assert!(s.svm.get_account(&s.vault).is_none());
    }

    #[test]
    fn test_make_then_refund_full_cycle() {
        let initial_balance = 2_000_000_000u64;
        let amount_to_give = 800_000_000u64;

        let mut escrow_setup = setup_make(100_000_000, amount_to_give, initial_balance, 3600);

        let vault_balance = read_token_balance(&escrow_setup.svm, &escrow_setup.vault);
        assert_eq!(vault_balance, amount_to_give);

        let refund_data: Vec<u8> = vec![2u8];
        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(escrow_setup.maker.pubkey(), true),
                AccountMeta::new_readonly(escrow_setup.mint_a, false),
                AccountMeta::new(escrow_setup.escrow, false),
                AccountMeta::new(escrow_setup.maker_ata_a, false),
                AccountMeta::new(escrow_setup.vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            ],
            data: refund_data,
        };

        let msg = Message::new(&[ix], Some(&escrow_setup.maker.pubkey()));
        let blockhash = escrow_setup.svm.latest_blockhash();
        let tx = Transaction::new(&[&escrow_setup.maker], msg, blockhash);
        escrow_setup.svm.send_transaction(tx).unwrap();

        let final_balance = read_token_balance(&escrow_setup.svm, &escrow_setup.maker_ata_a);
        assert_eq!(final_balance, initial_balance);
    }

    #[test]
    fn test_make_then_take_full_cycle() {
        let maker_mint = 5_000_000_000u64;
        let taker_mint = 5_000_000_000u64;
        let amount_to_receive = 1_000_000_000u64;
        let amount_to_give = 2_000_000_000u64;

        let s = setup_take(amount_to_receive, amount_to_give, maker_mint, taker_mint);

        assert_eq!(
            read_token_balance(&s.svm, &s.maker_ata_b),
            amount_to_receive
        );
        assert_eq!(read_token_balance(&s.svm, &s.taker_ata_a), amount_to_give);
        assert_eq!(
            read_token_balance(&s.svm, &s.maker_ata_a),
            maker_mint - amount_to_give
        );
        assert_eq!(
            read_token_balance(&s.svm, &s.taker_ata_b),
            taker_mint - amount_to_receive
        );
    }
}
