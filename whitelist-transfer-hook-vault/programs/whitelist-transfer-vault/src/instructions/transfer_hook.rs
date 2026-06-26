use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction};

/// Accounts required for the transfer hook execution.
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
    /// Source account's whitelist entry.
    pub source_whitelist_entry: Account<'info, WhitelistEntry>,
    /// Destination account's whitelist entry.
    pub destination_whitelist_entry: Account<'info, WhitelistEntry>,
}

/// Executes the transfer hook validation logic.
/// Verifies that all accounts match the expected TLV recipe.
pub fn execute_hook(ctx: Context<Execute>, amount: u64) -> Result<()> {
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
