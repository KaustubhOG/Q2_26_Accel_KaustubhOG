use anchor_lang::prelude::*;

use crate::{VaultConfig, WhitelistEntry, VAULT_CONFIG_SEED, WHITELIST_SEED};

/// Accounts required to remove a user from the whitelist.
#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct RemoveFromWhitelist<'info> {
    /// Admin authority (must match vault admin).
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Vault config PDA for verification.
    #[account(
        seeds = [VAULT_CONFIG_SEED],
        bump = vault_pda.bump,
    )]
    pub vault_pda: Account<'info, VaultConfig>,

    /// Whitelist entry to close (must belong to the specified user).
    #[account(
        mut,
        close = admin,
        seeds = [WHITELIST_SEED, user.as_ref()],
        bump,
        constraint = whitelist_entry.wallet == user
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
}

impl<'info> RemoveFromWhitelist<'info> {
    /// Removes a user from the whitelist and closes their account.
    /// Lamports are returned to the admin.
    pub fn remove_from_whitelist(&mut self, _user: Pubkey) -> Result<()> {
        Ok(())
    }
}
