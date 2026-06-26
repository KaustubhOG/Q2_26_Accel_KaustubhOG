use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Signer, Seed},
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_token::instructions::Transfer;

use crate::{
    constants::SECONDS_TO_DAYS,
    state::{Contributor, Fundraiser},
};

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

pub struct Refund;

impl Refund {
    pub fn process(program_id: &Address, accounts: &[AccountView]) -> ProgramResult {
        let [contributor, maker, mint_to_raise, fundraiser, contributor_account, contributor_ata, vault, _token_program, _system_program, _remaining @ ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !contributor.is_signer() || !contributor.is_writable() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !fundraiser.is_writable() || !contributor_account.is_writable() || !vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate fundraiser account
        if Fundraiser::discriminator(fundraiser) != crate::state::FUNDRAISER_DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }

        let fundraiser_maker = Fundraiser::maker(fundraiser);
        let amount_to_raise = Fundraiser::amount_to_raise(fundraiser);
        let time_started = Fundraiser::time_started(fundraiser);
        let duration = Fundraiser::duration(fundraiser);
        let current_amount = Fundraiser::current_amount(fundraiser);
        let fundraiser_bump = Fundraiser::bump(fundraiser);

        // Validate maker matches fundraiser
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

        // Validate contributor account PDA
        let (contributor_pda, _) = Address::derive_program_address(
            &[
                b"contributor",
                fundraiser.address().as_ref(),
                contributor.address().as_ref(),
            ],
            program_id,
        )
        .ok_or(ProgramError::InvalidSeeds)?;

        if *contributor_account.address() != contributor_pda {
            return Err(ProgramError::InvalidSeeds);
        }

        // Validate vault ATA
        let vault_ata = find_associated_token_address(&expected_pda, mint_to_raise.address());
        if *vault.address() != vault_ata {
            return Err(ProgramError::InvalidSeeds);
        }

        // FIXED: fundraiser must have ended (duration elapsed)
        let clock = Clock::get()?;
        let elapsed_days = ((clock.unix_timestamp - time_started) / SECONDS_TO_DAYS) as u8;
        if elapsed_days < duration {
            return Err(ProgramError::Custom(6005));
        }

        // Check if target was NOT met
        let vault_data = vault.try_borrow()?;
        if vault_data.len() < 72 {
            return Err(ProgramError::InvalidAccountData);
        }
        let vault_balance = u64::from_le_bytes(vault_data[64..72].try_into().unwrap());
        drop(vault_data);

        if vault_balance >= amount_to_raise {
            return Err(ProgramError::Custom(6001));
        }

        // Get contributor's amount
        let contributor_amount = Contributor::amount(contributor_account);

        // Transfer tokens from vault back to contributor via PDA-signed CPI
        let bump_byte = [fundraiser_bump];
        let signer_seeds = [
            Seed::from(b"fundraiser".as_ref()),
            Seed::from(maker.address().as_ref()),
            Seed::from(bump_byte.as_ref()),
        ];
        let signers = [Signer::from(signer_seeds.as_ref())];

        Transfer {
            from: vault,
            to: contributor_ata,
            authority: fundraiser,
            amount: contributor_amount,
        }
        .invoke_signed(&signers)?;

        // Update fundraiser current_amount
        Fundraiser::set_current_amount(fundraiser, current_amount - contributor_amount);

        // Close contributor account (return rent to contributor)
        let contrib_data_len = contributor_account.data_len();
        let rent = Rent::get()?;
        let lamports = rent.try_minimum_balance(contrib_data_len)?;

        contributor.set_lamports(contributor.lamports() + lamports);
        contributor_account.set_lamports(contributor_account.lamports() - lamports);
        contributor_account.close()?;

        Ok(())
    }
}
