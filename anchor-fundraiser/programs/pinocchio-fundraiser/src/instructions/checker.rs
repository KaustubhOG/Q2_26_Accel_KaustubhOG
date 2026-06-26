use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Signer, Seed},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_token::instructions::Transfer;

use crate::state::Fundraiser;

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

pub struct CheckContributions;

impl CheckContributions {
    pub fn process(program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
        let [maker, mint_to_raise, fundraiser, vault, maker_ata, _token_program, _system_program, _associated_token_program, _remaining @ ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !maker.is_signer() || !maker.is_writable() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !fundraiser.is_writable() || !vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate fundraiser account
        if Fundraiser::discriminator(fundraiser) != crate::state::FUNDRAISER_DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }

        let fundraiser_maker = Fundraiser::maker(fundraiser);
        let amount_to_raise = Fundraiser::amount_to_raise(fundraiser);
        let fundraiser_bump = Fundraiser::bump(fundraiser);

        // Validate maker is the fundraiser creator
        let expected_maker = Address::new_from_array(*fundraiser_maker);
        if *maker.address() != expected_maker {
            return Err(ProgramError::InvalidAccountData);
        }

        // Derive and validate fundraiser PDA
        let (expected_pda, _) = Address::derive_program_address(
            &[b"fundraiser", maker.address().as_ref()],
            program_id,
        )
        .ok_or(ProgramError::InvalidSeeds)?;

        if *fundraiser.address() != expected_pda {
            return Err(ProgramError::InvalidSeeds);
        }

        // Validate vault ATA
        let vault_ata = find_associated_token_address(&expected_pda, mint_to_raise.address());
        if *vault.address() != vault_ata {
            return Err(ProgramError::InvalidSeeds);
        }

        // Read vault token balance (SPL Token layout: amount at offset 64)
        let vault_data = vault.try_borrow()?;
        if vault_data.len() < 72 {
            return Err(ProgramError::InvalidAccountData);
        }
        let vault_balance = u64::from_le_bytes(vault_data[64..72].try_into().unwrap());
        drop(vault_data);

        // Check if target has been met
        if vault_balance < amount_to_raise {
            return Err(ProgramError::Custom(6000));
        }

        // Transfer all tokens from vault to maker via PDA-signed CPI
        let bump_byte = [fundraiser_bump];
        let signer_seeds = [
            Seed::from(b"fundraiser".as_ref()),
            Seed::from(maker.address().as_ref()),
            Seed::from(bump_byte.as_ref()),
        ];
        let signers = [Signer::from(signer_seeds.as_ref())];

        Transfer {
            from: vault,
            to: maker_ata,
            authority: fundraiser,
            amount: vault_balance,
        }
        .invoke_signed(&signers)?;

        // Close fundraiser account (return rent to maker)
        let fund_data_len = fundraiser.data_len();
        let rent = Rent::get()?;
        let lamports = rent.try_minimum_balance(fund_data_len)?;

        maker.set_lamports(maker.lamports() + lamports);
        fundraiser.set_lamports(fundraiser.lamports() - lamports);
        fundraiser.close()?;

        Ok(())
    }
}
