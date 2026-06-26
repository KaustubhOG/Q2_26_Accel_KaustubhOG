use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_token::state::Account;

use crate::error::{EscrowError, log_error};
use crate::state::Escrow;

pub fn process_refund_instruction(
    accounts: &mut [AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        maker,
        _mint_a,
        escrow,
        maker_ata_a,
        vault,
        _system_program,
        _token_program,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        let e = EscrowError::MissingRequiredSignature;
        log_error(&e);
        return Err(e.into());
    }

    if !escrow.owned_by(&crate::ID) {
        let e = EscrowError::IncorrectProgramId;
        log_error(&e);
        return Err(e.into());
    }

    let (bump, amount_to_give, maker_address, maker_raw, mint_a_addr) = {
        let escrow_state = Escrow::from_account_info(escrow)?;
        (
            escrow_state.bump,
            escrow_state.amount_to_give(),
            *escrow_state.maker(),
            *escrow_state.maker_raw(),
            *escrow_state.mint_a(),
        )
    };

    if maker.address() != &maker_address {
        let e = EscrowError::UnauthorizedRefund;
        log_error(&e);
        return Err(e.into());
    }

    // validate vault: correct mint and owned by escrow PDA
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

    // validate maker_ata_a: must be owned by maker and hold mint_a
    // prevents attacker passing arbitrary ATA to redirect refunded tokens
    {
        let maker_ata_state = Account::from_account_view(maker_ata_a)?;
        if maker_ata_state.owner() != maker.address() {
            let e = EscrowError::IllegalOwner;
            log_error(&e);
            return Err(e.into());
        }
        if maker_ata_state.mint() != &mint_a_addr {
            let e = EscrowError::InvalidAccountData;
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

    pinocchio_token::instructions::Transfer::new(vault, maker_ata_a, escrow, amount_to_give)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    pinocchio_token::instructions::CloseAccount::new(vault, maker, escrow)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    // transfer escrow rent lamports to maker before close() zeros them
    maker.set_lamports(maker.lamports() + escrow.lamports());
    escrow.set_lamports(0);
    escrow.close()?;

    pinocchio_log::log!("ESCROW_REFUND give={}", amount_to_give);

    Ok(())
}