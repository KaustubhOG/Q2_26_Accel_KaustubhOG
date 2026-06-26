#[cfg(test)]
mod tests {

    use {
        anchor_lang::solana_program::clock::Clock as SolClock,
        anchor_lang::{
            prelude::msg,
            solana_program::{instruction::Instruction, program_pack::Pack},
            system_program::ID as SYSTEM_PROGRAM_ID,
            AccountDeserialize, InstructionData, ToAccountMetas,
        },
        anchor_spl::{
            associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
            token::spl_token,
        },
        litesvm::LiteSVM,
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
        },
        solana_account::Account,
        solana_keypair::Keypair,
        solana_message::Message,
        solana_pubkey::Pubkey,
        solana_rpc_client::rpc_client::RpcClient,
        solana_signer::Signer,
        solana_transaction::Transaction,
        std::str::FromStr,
    };

    const AIRDROP_LAMPORTS: u64 = 10_000_000_000;
    const MINT_DECIMALS: u8 = 6;
    const INITIAL_MINT_AMOUNT: u64 = 1_000_000_000;
    const DEVNET_ACCOUNT: &str = "DRYvf71cbF2s5wgaJQvAGkghMkRcp5arvsK2w97vXhi2";

    struct EscrowTestContext {
        svm: LiteSVM,
        maker: Keypair,
        // raw secret stored separately so helpers can reconstruct a signer
        // without fighting the borrow checker on ctx.maker vs ctx.svm
        maker_secret: [u8; 32],
        taker: Keypair,
        taker_secret: [u8; 32],
        mint_a: Pubkey,
        mint_b: Pubkey,
        maker_ata_a: Pubkey,
        maker_ata_b: Pubkey,
        taker_ata_a: Pubkey,
        taker_ata_b: Pubkey,
        escrow: Pubkey,
        vault: Pubkey,
        seed: u64,
    }

    fn base_svm(recipient: &Pubkey) -> LiteSVM {
        let program_id = anchor_escrow_q2_2026::id();
        let mut svm = LiteSVM::new();

        let bytes = include_bytes!("../../../target/deploy/anchor_escrow_q2_2026.so");
        svm.add_program(program_id, bytes).unwrap();
        svm.airdrop(recipient, AIRDROP_LAMPORTS).unwrap();

        // pull the devnet account into our local svm so we dont need a live validator
        let rpc = RpcClient::new("https://api.devnet.solana.com");
        let addr = Pubkey::from_str(DEVNET_ACCOUNT).unwrap();
        if let Ok(fetched) = rpc.get_account(&addr) {
            svm.set_account(
                addr,
                Account {
                    lamports: fetched.lamports,
                    data: fetched.data,
                    owner: Pubkey::from(fetched.owner.to_bytes()),
                    executable: fetched.executable,
                    rent_epoch: fetched.rent_epoch,
                },
            )
            .unwrap();
        }

        svm
    }

    fn derive_escrow_accounts(maker: &Pubkey, seed: u64, mint_a: &Pubkey) -> (Pubkey, Pubkey) {
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
            &anchor_escrow_q2_2026::id(),
        )
        .0;
        // vault is just the ATA of the escrow PDA for mint_a
        let vault = associated_token::get_associated_token_address(&escrow, mint_a);
        (escrow, vault)
    }

    fn send_ix(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair], payer: &Pubkey) -> u64 {
        svm.expire_blockhash(); // forces a fresh blockhash → different tx signature
        let blockhash = svm.latest_blockhash();
        let message = Message::new(&[ix], Some(payer));
        let tx = Transaction::new(signers, message, blockhash);
        let meta = svm.send_transaction(tx).expect("transaction failed");
        meta.compute_units_consumed
    }

    fn setup_full_context(seed: u64) -> EscrowTestContext {
        let maker = Keypair::new();
        let taker = Keypair::new();

        let mut svm = base_svm(&maker.pubkey());
        svm.airdrop(&taker.pubkey(), AIRDROP_LAMPORTS).unwrap();

        // maker owns mint_a, taker owns mint_b   simulates two separate token issuers
        let mint_a = CreateMint::new(&mut svm, &maker)
            .decimals(MINT_DECIMALS)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut svm, &taker)
            .decimals(MINT_DECIMALS)
            .authority(&taker.pubkey())
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // fund each side with their respective token so the swap has something to work with
        MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, INITIAL_MINT_AMOUNT)
            .send()
            .unwrap();

        MintTo::new(&mut svm, &taker, &mint_b, &taker_ata_b, INITIAL_MINT_AMOUNT)
            .send()
            .unwrap();

        let (escrow, vault) = derive_escrow_accounts(&maker.pubkey(), seed, &mint_a);

        let maker_secret: [u8; 32] = maker.to_bytes()[..32].try_into().unwrap();
        let taker_secret: [u8; 32] = taker.to_bytes()[..32].try_into().unwrap();

        EscrowTestContext {
            svm,
            maker,
            maker_secret,
            taker,
            taker_secret,
            mint_a,
            mint_b,
            maker_ata_a,
            maker_ata_b,
            taker_ata_a,
            taker_ata_b,
            escrow,
            vault,
            seed,
        }
    }

    fn execute_make(ctx: &mut EscrowTestContext, deposit: u64, receive: u64) -> u64 {
        let ix = Instruction {
            program_id: anchor_escrow_q2_2026::id(),
            accounts: anchor_escrow_q2_2026::accounts::Make {
                maker: ctx.maker.pubkey(),
                mint_a: ctx.mint_a,
                mint_b: ctx.mint_b,
                maker_ata_a: ctx.maker_ata_a,
                escrow: ctx.escrow,
                vault: ctx.vault,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: anchor_escrow_q2_2026::instruction::Make {
                seed: ctx.seed,
                deposit,
                receive,
            }
            .data(),
        };

        let maker_pubkey = ctx.maker.pubkey();
        // reconstruct from stored secret   to_bytes() returns [secret(32) | public(32)]
        let maker_ref = Keypair::new_from_array(ctx.maker_secret);
        send_ix(&mut ctx.svm, ix, &[&maker_ref], &maker_pubkey)
    }

    fn execute_take(ctx: &mut EscrowTestContext) -> u64 {
        let ix = Instruction {
            program_id: anchor_escrow_q2_2026::id(),
            accounts: anchor_escrow_q2_2026::accounts::Take {
                taker: ctx.taker.pubkey(),
                maker: ctx.maker.pubkey(),
                mint_a: ctx.mint_a,
                mint_b: ctx.mint_b,
                taker_ata_a: ctx.taker_ata_a,
                taker_ata_b: ctx.taker_ata_b,
                maker_ata_b: ctx.maker_ata_b,
                escrow: ctx.escrow,
                vault: ctx.vault,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: anchor_escrow_q2_2026::instruction::Take {}.data(),
        };

        let taker_pubkey = ctx.taker.pubkey();
        let taker_ref = Keypair::new_from_array(ctx.taker_secret);
        send_ix(&mut ctx.svm, ix, &[&taker_ref], &taker_pubkey)
    }

    fn execute_refund(ctx: &mut EscrowTestContext) -> u64 {
        let ix = Instruction {
            program_id: anchor_escrow_q2_2026::id(),
            accounts: anchor_escrow_q2_2026::accounts::Refund {
                maker: ctx.maker.pubkey(),
                mint_a: ctx.mint_a,
                maker_ata_a: ctx.maker_ata_a,
                escrow: ctx.escrow,
                vault: ctx.vault,
                // no associated_token_program here   refund only closes the vault via token_program CPI
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: anchor_escrow_q2_2026::instruction::Refund {}.data(),
        };

        let maker_pubkey = ctx.maker.pubkey();
        let maker_ref = Keypair::new_from_array(ctx.maker_secret);
        send_ix(&mut ctx.svm, ix, &[&maker_ref], &maker_pubkey)
    }

    fn fetch_escrow_state(svm: &LiteSVM, escrow: &Pubkey) -> anchor_escrow_q2_2026::state::Escrow {
        let account = svm.get_account(escrow).expect("escrow account not found");
        anchor_escrow_q2_2026::state::Escrow::try_deserialize(&mut account.data.as_ref())
            .expect("failed to deserialize escrow state")
    }

    fn token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
        let account = svm.get_account(ata).expect("token account not found");
        spl_token::state::Account::unpack(&account.data)
            .expect("failed to unpack token account")
            .amount
    }

    #[test]
    fn test_make() {
        let seed = 100u64;
        let deposit = 500_000u64;
        let receive = 250_000u64;

        let mut ctx = setup_full_context(seed);
        let maker_balance_before = token_balance(&ctx.svm, &ctx.maker_ata_a);

        let cu = execute_make(&mut ctx, deposit, receive);
        msg!("Make CUs consumed: {}", cu);

        // vault should hold exactly what maker deposited, owned by the escrow PDA
        let vault_account = ctx.svm.get_account(&ctx.vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, deposit);
        assert_eq!(vault_data.owner, ctx.escrow);
        assert_eq!(vault_data.mint, ctx.mint_a);

        // maker's ATA must reflect the debit
        assert_eq!(
            maker_balance_before - token_balance(&ctx.svm, &ctx.maker_ata_a),
            deposit
        );

        // escrow state must record all the deal terms correctly
        let escrow_state = fetch_escrow_state(&ctx.svm, &ctx.escrow);
        assert_eq!(escrow_state.seed, seed);
        assert_eq!(escrow_state.maker, ctx.maker.pubkey());
        assert_eq!(escrow_state.mint_a, ctx.mint_a);
        assert_eq!(escrow_state.mint_b, ctx.mint_b);
        assert_eq!(escrow_state.receive, receive);
    }

    #[test]
    fn test_take() {
        let seed = 200u64;
        let deposit = 1_000u64;
        let receive = 2_000u64;

        let mut ctx = setup_full_context(seed);
        execute_make(&mut ctx, deposit, receive);

        // snapshot balances before take so we can measure exact deltas
        let maker_ata_b_before = token_balance(&ctx.svm, &ctx.maker_ata_b);
        let taker_ata_a_before = token_balance(&ctx.svm, &ctx.taker_ata_a);
        let taker_ata_b_before = token_balance(&ctx.svm, &ctx.taker_ata_b);

        let cu = execute_take(&mut ctx);
        msg!("Take CUs consumed: {}", cu);

        // taker receives the vaulted mint_a
        assert_eq!(
            token_balance(&ctx.svm, &ctx.taker_ata_a) - taker_ata_a_before,
            deposit
        );
        // maker receives the agreed mint_b amount from taker
        assert_eq!(
            token_balance(&ctx.svm, &ctx.maker_ata_b) - maker_ata_b_before,
            receive
        );
        // taker paid exactly the receive amount
        assert_eq!(
            taker_ata_b_before - token_balance(&ctx.svm, &ctx.taker_ata_b),
            receive
        );

        // both vault and escrow must be closed   lamports returned to maker
        assert!(ctx.svm.get_account(&ctx.vault).is_none());
        assert!(ctx.svm.get_account(&ctx.escrow).is_none());
    }

    #[test]
    fn test_refund() {
        let seed = 300u64;
        let deposit = 750_000u64;
        let receive = 500_000u64;

        let mut ctx = setup_full_context(seed);
        execute_make(&mut ctx, deposit, receive);

        let maker_balance_before = token_balance(&ctx.svm, &ctx.maker_ata_a);

        let cu = execute_refund(&mut ctx);
        msg!("Refund CUs consumed: {}", cu);

        // maker should get back exactly what they put in, nothing more nothing less
        assert_eq!(
            token_balance(&ctx.svm, &ctx.maker_ata_a) - maker_balance_before,
            deposit
        );

        // vault and escrow must be closed on refund just like on take
        assert!(ctx.svm.get_account(&ctx.vault).is_none());
        assert!(ctx.svm.get_account(&ctx.escrow).is_none());
    }

    #[test]
    fn test_full_round_trip() {
        let seed = 400u64;
        let deposit = 5_000u64;
        let receive = 3_000u64;

        let mut ctx = setup_full_context(seed);

        let maker_a_initial = token_balance(&ctx.svm, &ctx.maker_ata_a);
        let maker_b_initial = token_balance(&ctx.svm, &ctx.maker_ata_b);
        let taker_a_initial = token_balance(&ctx.svm, &ctx.taker_ata_a);
        let taker_b_initial = token_balance(&ctx.svm, &ctx.taker_ata_b);

        execute_make(&mut ctx, deposit, receive);
        execute_take(&mut ctx);

        // maker lost deposit of mint_a and gained receive of mint_b
        assert_eq!(
            maker_a_initial - token_balance(&ctx.svm, &ctx.maker_ata_a),
            deposit
        );
        assert_eq!(
            token_balance(&ctx.svm, &ctx.maker_ata_b) - maker_b_initial,
            receive
        );

        // taker gained deposit of mint_a and lost receive of mint_b
        assert_eq!(
            token_balance(&ctx.svm, &ctx.taker_ata_a) - taker_a_initial,
            deposit
        );
        assert_eq!(
            taker_b_initial - token_balance(&ctx.svm, &ctx.taker_ata_b),
            receive
        );

        // total supply of each token must be conserved   nothing minted or burned mid-swap
        assert_eq!(
            maker_a_initial + taker_a_initial,
            token_balance(&ctx.svm, &ctx.maker_ata_a) + token_balance(&ctx.svm, &ctx.taker_ata_a)
        );
        assert_eq!(
            maker_b_initial + taker_b_initial,
            token_balance(&ctx.svm, &ctx.maker_ata_b) + token_balance(&ctx.svm, &ctx.taker_ata_b)
        );
    }

    #[test]
    fn test_refund_then_remake_same_seed() {
        let seed = 500u64;
        let deposit = 1_000u64;
        let receive = 1_000u64;

        let mut ctx = setup_full_context(seed);

        execute_make(&mut ctx, deposit, receive);
        assert!(ctx.svm.get_account(&ctx.escrow).is_some());

        execute_refund(&mut ctx);
        // after refund the PDA is gone, seed is free to reuse
        assert!(ctx.svm.get_account(&ctx.escrow).is_none());

        // reusing the same seed must work   the old account was properly closed
        execute_make(&mut ctx, deposit, receive);
        assert!(ctx.svm.get_account(&ctx.escrow).is_some());

        let escrow_state = fetch_escrow_state(&ctx.svm, &ctx.escrow);
        assert_eq!(escrow_state.seed, seed);
        assert_eq!(escrow_state.maker, ctx.maker.pubkey());
    }

    #[test]
    fn test_make_minimum_deposit() {
        let seed = 600u64;

        let mut ctx = setup_full_context(seed);
        execute_make(&mut ctx, 1, 1);

        // smallest possible deposit   make sure the program doesnt floor or skip it
        assert_eq!(token_balance(&ctx.svm, &ctx.vault), 1);

        let escrow_state = fetch_escrow_state(&ctx.svm, &ctx.escrow);
        assert_eq!(escrow_state.receive, 1);
    }

    #[test]
    fn test_make_full_supply() {
        let seed = 700u64;

        let mut ctx = setup_full_context(seed);
        execute_make(&mut ctx, INITIAL_MINT_AMOUNT, INITIAL_MINT_AMOUNT);

        // entire supply goes into vault   maker ATA should be zeroed out
        assert_eq!(token_balance(&ctx.svm, &ctx.vault), INITIAL_MINT_AMOUNT);
        assert_eq!(token_balance(&ctx.svm, &ctx.maker_ata_a), 0);
    }

    #[test]
    fn test_multiple_concurrent_escrows() {
        let seed_1 = 800u64;
        let seed_2 = 801u64;

        let mut ctx = setup_full_context(seed_1);
        // different seed = different PDA, both should coexist without interfering
        let (escrow_2, vault_2) = derive_escrow_accounts(&ctx.maker.pubkey(), seed_2, &ctx.mint_a);

        execute_make(&mut ctx, 100, 50);

        let maker_ref = Keypair::new_from_array(ctx.maker_secret);
        let ix2 = Instruction {
            program_id: anchor_escrow_q2_2026::id(),
            accounts: anchor_escrow_q2_2026::accounts::Make {
                maker: ctx.maker.pubkey(),
                mint_a: ctx.mint_a,
                mint_b: ctx.mint_b,
                maker_ata_a: ctx.maker_ata_a,
                escrow: escrow_2,
                vault: vault_2,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: anchor_escrow_q2_2026::instruction::Make {
                seed: seed_2,
                deposit: 200,
                receive: 100,
            }
            .data(),
        };
        send_ix(&mut ctx.svm, ix2, &[&maker_ref], &ctx.maker.pubkey());

        assert!(ctx.svm.get_account(&ctx.escrow).is_some());
        assert!(ctx.svm.get_account(&escrow_2).is_some());

        // each vault holds only its own deposit
        assert_eq!(token_balance(&ctx.svm, &ctx.vault), 100);
        assert_eq!(token_balance(&ctx.svm, &vault_2), 200);

        let state_1 = fetch_escrow_state(&ctx.svm, &ctx.escrow);
        let state_2 = fetch_escrow_state(&ctx.svm, &escrow_2);
        assert_eq!(state_1.seed, seed_1);
        assert_eq!(state_2.seed, seed_2);
    }

    #[test]
    fn test_instruction_discriminators_unique() {
        let make_data = anchor_escrow_q2_2026::instruction::Make {
            seed: 1,
            deposit: 1,
            receive: 1,
        }
        .data();
        let take_data = anchor_escrow_q2_2026::instruction::Take {}.data();
        let refund_data = anchor_escrow_q2_2026::instruction::Refund {}.data();

        // if any two share a discriminator the router will silently call the wrong instruction
        // compare the full serialized data since anchor uses custom discriminators here
        assert_ne!(make_data, take_data);
        assert_ne!(make_data, refund_data);
        assert_ne!(take_data, refund_data);
    }
}
