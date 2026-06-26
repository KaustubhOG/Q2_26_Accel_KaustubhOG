use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};
use pinocchio_token::state::Account;

use crate::error::{EscrowError, log_error};
use crate::state::Escrow;

pub fn process_take_instruction(
    accounts: &mut [AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        taker,
        maker,
        _mint_a,
        _mint_b,
        escrow,
        taker_ata_a,
        taker_ata_b,
        maker_ata_b,
        vault,
        _system_program,
        _token_program,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !taker.is_signer() {
        let e = EscrowError::MissingRequiredSignature;
        log_error(&e);
        return Err(e.into());
    }

    if !escrow.owned_by(&crate::ID) {
        let e = EscrowError::IncorrectProgramId;
        log_error(&e);
        return Err(e.into());
    }

    let (bump, amount_to_give, amount_to_receive, maker_raw, mint_a_addr, expiry) = {
        let escrow_state = Escrow::from_account_info(escrow)?;
        (
            escrow_state.bump,
            escrow_state.amount_to_give(),
            escrow_state.amount_to_receive(),
            *escrow_state.maker_raw(),
            *escrow_state.mint_a(),
            escrow_state.timestamp(),
        )
    };

    let clock = Clock::get()?;
    if clock.unix_timestamp > expiry {
        let e = EscrowError::EscrowExpired;
        log_error(&e);
        return Err(e.into());
    }

    {
        let vault_state = Account::from_account_view(vault)?;
        if vault_state.mint() != &mint_a_addr {
            let e = EscrowError::InvalidVault;
            log_error(&e);
            return Err(e.into());
        }
        if vault_state.owner() != escrow.address() {
            let e = EscrowError::InvalidVault;
            log_error(&e);
            return Err(e.into());
        }
    }

    // scoped so borrow on taker_ata_b drops before CPIs
    {
        let taker_b_state = Account::from_account_view(taker_ata_b)?;
        if taker_b_state.amount() < amount_to_receive {
            let e = EscrowError::InsufficientFunds;
            log_error(&e);
            return Err(e.into());
        }
    }

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"escrow"),
        Seed::from(maker_raw.as_ref()),
        Seed::from(bump_bytes.as_ref()),
    ];

    pinocchio_token::instructions::Transfer::new(
        taker_ata_b,
        maker_ata_b,
        taker,
        amount_to_receive,
    )
    .invoke()?;

    pinocchio_token::instructions::Transfer::new(vault, taker_ata_a, escrow, amount_to_give)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    pinocchio_token::instructions::CloseAccount::new(vault, maker, escrow)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    maker.set_lamports(maker.lamports() + escrow.lamports());
    escrow.set_lamports(0);
    escrow.close()?;

    pinocchio_log::log!(
        "ESCROW_TAKE receive={} give={}",
        amount_to_receive,
        amount_to_give
    );

    Ok(())
}