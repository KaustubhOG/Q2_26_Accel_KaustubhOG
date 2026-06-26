use anchor_lang::prelude::*;

use crate::{VaultConfig, WhitelistEntry, VAULT_CONFIG_SEED, WHITELIST_SEED};

/// Accounts required to add a user to the whitelist.
#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct AddToWhitelist<'info> {
    /// Admin authority (must match vault admin).
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Vault config PDA for verification.
    #[account(
        seeds = [VAULT_CONFIG_SEED],
        bump = vault_pda.bump,
    )]
    pub vault_pda: Account<'info, VaultConfig>,

    /// Whitelist entry PDA for the user (created if not exists).
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + WhitelistEntry::INIT_SPACE,
        seeds = [WHITELIST_SEED, user.as_ref()],
        bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    pub system_program: Program<'info, System>,
}

impl<'info> AddToWhitelist<'info> {
    /// Adds a user to the whitelist, preserving existing balance if re-adding.
    pub fn add_to_whitelist(&mut self, bumps: AddToWhitelistBumps, user: Pubkey) -> Result<()> {
        self.whitelist_entry.set_inner(WhitelistEntry {
            wallet: user,
            bump: bumps.whitelist_entry,
            balance: self.whitelist_entry.balance,
        });

        Ok(())
    }
}
