use anchor_lang::prelude::*;

// One account is created per whitelisted address.
// Seeds: ["whitelist", address]    existence of this account IS the whitelist check.
// Storing the bump avoids a find_program_address call on every transfer hook invocation.
#[account]
pub struct WhitelistEntry {
    pub bump: u8,
}

impl WhitelistEntry {
    pub const LEN: usize = 8 + 1; // discriminator + bump
}