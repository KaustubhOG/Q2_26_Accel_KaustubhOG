use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{
    VaultConfig, WhitelistEntry, MINT_SEED, TRANSFER_HOOK_PROGRAM_ID, VAULT_CONFIG_SEED,
    WHITELIST_SEED,
};

/// Accounts required to initialize the vault program.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The admin initializing the vault (pays for account creation).
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Vault configuration PDA storing admin, mint, and token account references.
    #[account(
        init,
        payer = admin,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [VAULT_CONFIG_SEED],
        bump
    )]
    pub vault_pda: Account<'info, VaultConfig>,

    /// Mint PDA with transfer hook extension pointing to the hook program.
    /// Also has permanent delegate extension for vault authority.
    #[account(
        init,
        payer = admin,
        seeds = [MINT_SEED],
        bump,
        mint::decimals = 6,
        mint::authority = vault_pda,
        mint::token_program = token_program,
        extensions::transfer_hook::authority = vault_pda,
        extensions::transfer_hook::program_id = TRANSFER_HOOK_PROGRAM_ID,
        extensions::permanent_delegate::delegate = vault_pda
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// Vault's associated token account (ATA) for holding deposited tokens.
    #[account(
        init,
        payer = admin,
        associated_token::mint = mint,
        associated_token::authority = vault_pda,
        associated_token::token_program = token_program,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Whitelist entry for the vault itself (allows vault to receive deposits).
    #[account(
        init,
        payer = admin,
        space = 8 + WhitelistEntry::INIT_SPACE,
        seeds = [WHITELIST_SEED, vault_pda.key().as_ref()],
        bump
    )]
    pub vault_whitelist_entry: Account<'info, WhitelistEntry>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    /// Initializes the vault config and vault's whitelist entry.
    pub fn initialize(&mut self, bumps: InitializeBumps) -> Result<()> {
        self.vault_pda.set_inner(VaultConfig {
            admin: self.admin.key(),
            mint: self.mint.key(),
            vault_token_account: self.vault_token_account.key(),
            bump: bumps.vault_pda,
            mint_bump: bumps.mint,
        });

        self.vault_whitelist_entry.set_inner(WhitelistEntry {
            wallet: self.vault_pda.key(),
            bump: bumps.vault_whitelist_entry,
            balance: 0,
        });
        Ok(())
    }
}
