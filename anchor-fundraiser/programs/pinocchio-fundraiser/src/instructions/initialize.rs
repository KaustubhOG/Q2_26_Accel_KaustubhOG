use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::InitializeAccount3;

use crate::{
    constants::MIN_AMOUNT_TO_RAISE,
    state::{Fundraiser, FUNDRAISER_SIZE},
};

/// Associated Token Account program ID
const ASSOCIATED_TOKEN_PROGRAM_ID: Address = Address::new_from_array([
    0x05, 0x48, 0x03, 0x05, 0x68, 0x50, 0x42, 0x08,
    0xED, 0xA3, 0x17, 0x23, 0xD0, 0x57, 0xA3, 0xBD,
    0xB8, 0x3A, 0x1D, 0x35, 0xF5, 0xC7, 0x48, 0x66,
    0xBB, 0x59, 0x15, 0x86, 0x9A, 0x86, 0xE6, 0x74,
]);

fn find_associated_token_address(wallet: &Address, mint: &Address) -> Address {
    Address::derive_program_address(
        &[
            wallet.as_ref(),
            pinocchio_token::ID.as_ref(),
            mint.as_ref(),
        ],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .unwrap()
    .0
}

pub struct Initialize;

impl Initialize {
    pub fn process(
        program_id: &Address,
        accounts: &[AccountView],
        amount: u64,
        duration: u8,
    ) -> ProgramResult {
        let [maker, mint_to_raise, fundraiser, vault, _system_program, _token_program, _associated_token_program, _remaining @ ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Validate signer and writable
        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !maker.is_writable() || !fundraiser.is_writable() || !vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate minimum amount
        let decimals = 9u32;
        if amount <= MIN_AMOUNT_TO_RAISE.pow(decimals) {
            return Err(ProgramError::Custom(6007));
        }

        // Derive fundraiser PDA
        let (fundraiser_pda, fundraiser_bump) = Address::derive_program_address(
            &[b"fundraiser", maker.address().as_ref()],
            program_id,
        )
        .ok_or(ProgramError::InvalidSeeds)?;

        if *fundraiser.address() != fundraiser_pda {
            return Err(ProgramError::InvalidSeeds);
        }

        // Derive vault ATA address
        let vault_ata = find_associated_token_address(&fundraiser_pda, mint_to_raise.address());
        if *vault.address() != vault_ata {
            return Err(ProgramError::InvalidSeeds);
        }

        // Create fundraiser account
        let rent = Rent::get()?;
        let space = FUNDRAISER_SIZE as u64;
        let lamports = rent.try_minimum_balance(space as usize)?;

        CreateAccount {
            from: maker,
            to: fundraiser,
            lamports,
            space,
            owner: &crate::ID,
        }
        .invoke()?;

        // Initialize fundraiser state
        let clock = pinocchio::sysvars::clock::Clock::get()?;

        // Get the bytes we need for maker and mint
        let maker_bytes: &[u8; 32] = maker.address().as_ref().try_into().unwrap();
        let mint_bytes: &[u8; 32] = mint_to_raise.address().as_ref().try_into().unwrap();

        Fundraiser::write(
            fundraiser,
            maker_bytes,
            mint_bytes,
            amount,
            0,
            clock.unix_timestamp,
            duration,
            fundraiser_bump,
        );

        // Create vault ATA (token account)
        let vault_lamports = rent.try_minimum_balance(165)?;

        CreateAccount {
            from: maker,
            to: vault,
            lamports: vault_lamports,
            space: 165,
            owner: &pinocchio_token::ID,
        }
        .invoke()?;

        // Initialize the token account
        InitializeAccount3 {
            account: vault,
            mint: mint_to_raise,
            owner: &fundraiser_pda,
        }
        .invoke()?;

        Ok(())
    }
}
