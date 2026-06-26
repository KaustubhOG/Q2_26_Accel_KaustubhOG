use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction};

use crate::VAULT_PROGRAM_ID;

/// Accounts required for the transfer hook execution (hook program side).
#[derive(Accounts)]
pub struct Execute<'info> {
    /// Source token account in the transfer.
    pub source_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Mint of the token being transferred.
    pub mint: InterfaceAccount<'info, Mint>,

    /// Destination token account in the transfer.
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Authority for the transfer (source owner or delegate).
    /// CHECK: Supplied by Token-2022 during transfer.
    pub authority: UncheckedAccount<'info>,

    /// Extra account meta list for validation.
    /// CHECK: TLV validation state for the transfer-hook interface.
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    /// Vault program ID (used as external PDA owner for whitelist resolution).
    /// CHECK: Verified by address constraint.
    #[account(address = VAULT_PROGRAM_ID)]
    pub vault_program: UncheckedAccount<'info>,

    /// Source account's whitelist entry (owned by vault program).
    /// CHECK: Deserialized manually because this account is owned by the vault program.
    pub source_whitelist_entry: UncheckedAccount<'info>,

    /// Destination account's whitelist entry (owned by vault program).
    /// CHECK: Deserialized manually because this account is owned by the vault program.
    pub destination_whitelist_entry: UncheckedAccount<'info>,
}

/// Executes the transfer hook validation logic.
/// Re-validates that all accounts match the expected TLV recipe.
pub fn execute(ctx: Context<Execute>, amount: u64) -> Result<()> {
    // Re-check the passed accounts against the TLV recipe. This catches clients/tests that
    // try to pass a different whitelist PDA than the one the hook expects.
    let instruction_data = TransferHookInstruction::Execute { amount }.pack();
    let validation_data = ctx.accounts.extra_account_meta_list.try_borrow_data()?;

    ExtraAccountMetaList::check_account_infos::<ExecuteInstruction>(
        &ctx.accounts.to_account_infos(),
        &instruction_data,
        ctx.program_id,
        &validation_data,
    )?;

    Ok(())
}