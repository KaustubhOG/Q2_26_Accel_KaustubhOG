use anchor_lang::prelude::*;

mod errors;
mod instructions;
mod state;

use instructions::*;

// the keypair used at deploy time. If you redeploy and get a new Program Id,
// update BOTH this line AND [programs.devnet] in Anchor.toml.
declare_id!("5sohXhaWnhRGq7q8P2s5jJnAr1b1Qr2EnLq9MFz7PfQV");

#[program]
pub mod anchor_amm_q4_25 {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
    ) -> Result<()> {
        ctx.accounts.init(seed, fee, authority, ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        ctx.accounts.deposit(amount, max_x, max_y)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        ctx.accounts.withdraw(amount, max_x, max_y)
    }

    pub fn swap(ctx: Context<Swap>, is_x: bool, amount_in: u64, min_amount_out: u64) -> Result<()> {
        ctx.accounts.swap(is_x, amount_in, min_amount_out)
    }

    /// Permissionless TWAP update     callable by anyone, including TukTuk cranks.
    pub fn update_twap(ctx: Context<UpdateTwap>) -> Result<()> {
        ctx.accounts.update_twap()
    }
}
