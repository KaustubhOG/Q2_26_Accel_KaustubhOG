use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    transfer_checked_with_hook, TokenTransferWithHookAccounts, VaultConfig, WhitelistEntry,
    TRANSFER_HOOK_PROGRAM_ID, VAULT_CONFIG_SEED, WHITELIST_SEED,
};

/// Accounts required to withdraw tokens from the vault.
#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// User withdrawing tokens (must be whitelisted).
    #[account(mut)]
    pub user: Signer<'info>,

    /// Vault config PDA (authority for vault token account).
    #[account(
        seeds = [VAULT_CONFIG_SEED],
        bump = vault_pda.bump,
    )]
    pub vault_pda: Account<'info, VaultConfig>,

    /// User's token account (destination of withdrawal).
    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
        token::token_program = token_program
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Vault's token account (source of withdrawal).
    #[account(mut)]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    /// The mint being withdrawn.
    pub mint: InterfaceAccount<'info, Mint>,

    /// User's whitelist entry (must exist and be mutable for balance update).
    #[account(
        mut,
        seeds = [WHITELIST_SEED, user.key().as_ref()],
        bump = user_whitelist_entry.bump,
    )]
    pub user_whitelist_entry: Account<'info, WhitelistEntry>,

    /// Vault's whitelist entry (source whitelist).
    #[account(
        seeds = [WHITELIST_SEED, vault_pda.key().as_ref()],
        bump = vault_whitelist_entry.bump,
    )]
    pub vault_whitelist_entry: Account<'info, WhitelistEntry>,

    /// Extra account meta list for transfer hook validation.
    /// CHECK: Validated by the transfer hook interface during token transfer.
    pub extra_account_meta_list: UncheckedAccount<'info>,

    /// Vault program ID (used by hook to derive whitelist PDAs).
    /// CHECK: Verified by address constraint.
    #[account(address = crate::ID)]
    pub vault_program: UncheckedAccount<'info>,

    /// Transfer hook program ID.
    /// CHECK: Verified by address constraint.
    #[account(address = TRANSFER_HOOK_PROGRAM_ID)]
    pub hook_program: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Withdraw<'info> {
    /// Withdraws `amount` tokens from vault to user via transfer hook.
    /// Updates user's whitelist balance on success.
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, &[self.vault_pda.bump]]];

        transfer_checked_with_hook(
            TokenTransferWithHookAccounts {
                source: self.vault_token_account.to_account_info(),
                mint: self.mint.to_account_info(),
                destination: self.user_token_account.to_account_info(),
                authority: self.vault_pda.to_account_info(),
                extra_meta_account_list: self.extra_account_meta_list.to_account_info(),
                vault_program: self.vault_program.to_account_info(),
                source_whitelist: self.vault_whitelist_entry.to_account_info(),
                destination_whitelist: self.user_whitelist_entry.to_account_info(),
                hook_program: self.hook_program.to_account_info(),
                token_program: self.token_program.to_account_info(),
                signer_seeds,
            },
            amount,
            self.mint.decimals,
        )?;

        self.user_whitelist_entry.balance = self
            .user_whitelist_entry
            .balance
            .checked_sub(amount)
            .unwrap();

        Ok(())
    }
}
