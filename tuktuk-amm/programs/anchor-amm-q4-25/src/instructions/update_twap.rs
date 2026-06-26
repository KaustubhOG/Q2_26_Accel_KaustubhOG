use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct UpdateTwap<'info> {
    /// Anyone can call this     permissionless so TukTuk crank can invoke it.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        mut,
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

impl<'info> UpdateTwap<'info> {
    pub fn update_twap(&mut self) -> Result<()> {
        require!(
            self.vault_x.amount > 0 && self.vault_y.amount > 0,
            AmmError::NoLiquidityInPool
        );

        let now = Clock::get()?.unix_timestamp;
        let elapsed = now - self.config.last_updated;

        // Spot price = (vault_y / vault_x) scaled by 1e6 for precision
        let spot_price = self
            .vault_y
            .amount
            .checked_mul(1_000_000)
            .ok_or(AmmError::Overflow)?
            .checked_div(self.vault_x.amount)
            .ok_or(AmmError::Underflow)?;

        // On first call (last_updated == 0), just set initial values
        if self.config.last_updated == 0 || elapsed <= 0 {
            self.config.last_price = spot_price;
            self.config.twap = spot_price;
            self.config.last_updated = now;
            return Ok(());
        }

        // Exponential moving average with a 60-second smoothing window:
        // twap = (spot * elapsed + twap * window) / (elapsed + window)
        let window: u64 = 60;
        let elapsed_u64 = elapsed as u64;

        let new_twap = spot_price
            .checked_mul(elapsed_u64)
            .ok_or(AmmError::Overflow)?
            .checked_add(
                self.config
                    .twap
                    .checked_mul(window)
                    .ok_or(AmmError::Overflow)?,
            )
            .ok_or(AmmError::Overflow)?
            .checked_div(elapsed_u64.checked_add(window).ok_or(AmmError::Overflow)?)
            .ok_or(AmmError::Underflow)?;

        self.config.last_price = spot_price;
        self.config.twap = new_twap;
        self.config.last_updated = now;

        msg!(
            "TWAP updated: spot={} twap={} elapsed={}s",
            spot_price,
            new_twap,
            elapsed
        );

        Ok(())
    }
}