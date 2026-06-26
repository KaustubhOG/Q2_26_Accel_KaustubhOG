use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::Transfer;

use crate::{
    constants::{MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER, SECONDS_TO_DAYS},
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

pub struct Contribute;

impl Contribute {
    pub fn process(
        program_id: &Address,
        accounts: &[AccountView],
        amount: u64,
    ) -> ProgramResult {
        let [contributor, mint_to_raise, fundraiser, contributor_account, contributor_ata, vault, _token_program, _system_program, _remaining @ ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !contributor.is_signer() || !contributor.is_writable() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !fundraiser.is_writable() || !contributor_account.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate fundraiser account
        if Fundraiser::discriminator(fundraiser) != crate::state::FUNDRAISER_DISCRIMINATOR {
            return Err(ProgramError::InvalidAccountData);
        }

        let fundraiser_maker = Fundraiser::maker(fundraiser);
        let fundraiser_mint = Fundraiser::mint_to_raise(fundraiser);
        let amount_to_raise = Fundraiser::amount_to_raise(fundraiser);
        let current_amount = Fundraiser::current_amount(fundraiser);
        let time_started = Fundraiser::time_started(fundraiser);
        let duration = Fundraiser::duration(fundraiser);

        // Validate mint matches
        let expected_mint = Address::new_from_array(*fundraiser_mint);
        if *mint_to_raise.address() != expected_mint {
            return Err(ProgramError::InvalidAccountData);
        }

        // Derive and validate fundraiser PDA
        let (expected_pda, _) = Address::derive_program_address(
            &[b"fundraiser", fundraiser_maker.as_ref()],
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

        // Derive and validate contributor account PDA
        let (contributor_pda, _bump) = Address::derive_program_address(
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

        // Check if contributor account exists
        let is_new = contributor_account.is_data_empty();

        // Validate contribution amount > 1 token (10^decimals)
        let min_contribution = 1u64.pow(9);
        if amount <= min_contribution {
            return Err(ProgramError::Custom(6003));
        }

        // Validate contribution <= 10% of target
        let max_contribution = (amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER;
        if amount > max_contribution {
            return Err(ProgramError::Custom(6002));
        }

        // FIXED: fundraiser should still be active (NOT ended)
        let clock = Clock::get()?;
        let elapsed_days = ((clock.unix_timestamp - time_started) / SECONDS_TO_DAYS) as u8;
        if elapsed_days >= duration {
            return Err(ProgramError::Custom(6006));
        }

        // Validate contributor hasn't exceeded their limit
        if !is_new {
            let contributor_amount = Contributor::amount(contributor_account);
            if contributor_amount > max_contribution
                || contributor_amount + amount > max_contribution
            {
                return Err(ProgramError::Custom(6004));
            }
        }

        // Transfer tokens from contributor to vault
        Transfer {
            from: contributor_ata,
            to: vault,
            authority: contributor,
            amount,
        }
        .invoke()?;

        // Update fundraiser current_amount
        Fundraiser::set_current_amount(fundraiser, current_amount + amount);

        // Initialize or update contributor account
        if is_new {
            let rent = Rent::get()?;
            let lamports = rent.try_minimum_balance(Contributor::SIZE)?;

            CreateAccount {
                from: contributor,
                to: contributor_account,
                lamports,
                space: Contributor::SIZE as u64,
                owner: program_id,
            }
            .invoke()?;

            Contributor::write(contributor_account, amount);
        } else {
            let current = Contributor::amount(contributor_account);
            Contributor::set_amount(contributor_account, current + amount);
        }

        Ok(())
    }
}
