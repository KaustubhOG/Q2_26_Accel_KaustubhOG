use anchor_lang::prelude::*;

/// Vault configuration account storing admin, mint, and vault token account info.
#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    /// Admin authority that manages the vault.
    pub admin: Pubkey,
    /// The mint address of the token managed by this vault.
    pub mint: Pubkey,
    /// The vault's associated token account holding deposited tokens.
    pub vault_token_account: Pubkey,
    /// PDA bump for the vault config account.
    pub bump: u8,
    /// PDA bump for the mint account.
    pub mint_bump: u8,
}

/// Whitelist entry tracking a user's eligibility and balance within the vault.
#[account]
#[derive(InitSpace)]
pub struct WhitelistEntry {
    /// The wallet address this entry belongs to.
    pub wallet: Pubkey,
    /// PDA bump for the whitelist entry account.
    pub bump: u8,
    /// Current token balance tracked for this whitelisted user.
    pub balance: u64,
}
