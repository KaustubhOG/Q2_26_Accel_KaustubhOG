use anchor_lang::prelude::*;

/// Whitelist entry tracking a wallet's eligibility (owned by vault program).
#[account]
#[derive(InitSpace)]
pub struct WhitelistEntry {
    /// The wallet address this entry belongs to.
    pub wallet: Pubkey,
    /// PDA bump for the whitelist entry account.
    pub bump: u8,
}

/// Configuration for the whitelist (currently unused but reserved for future use).
#[account]
#[derive(InitSpace)]
pub struct WhiteListConfig {
    /// The wallet this config belongs to.
    pub wallet: Pubkey,
    /// Optional mint restriction (None = all mints).
    pub mint: Option<Pubkey>,
    /// PDA bump for the config account.
    pub bump: u8,
}