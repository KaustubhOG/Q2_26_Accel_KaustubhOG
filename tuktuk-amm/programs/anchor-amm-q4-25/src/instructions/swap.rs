use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::ConstantProduct;

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,
    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount: u64, min: u64) -> Result<()> {
        // Step 1: Safety checks
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);
        require!(
            self.vault_x.amount > 0 && self.vault_y.amount > 0,
            AmmError::NoLiquidityInPool
        );

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            0, // LP supply not needed for swap calculations
            self.config.fee,
            Some(6),
        )
        .map_err(AmmError::from)?;

        let liquidity_pair = match is_x {
            true => constant_product_curve::LiquidityPair::X,
            false => constant_product_curve::LiquidityPair::Y,
        };

        let result = curve
            .swap(liquidity_pair, amount, min)
            .map_err(AmmError::from)?;

        let amount_out = result.withdraw;

        self.deposit_tokens(is_x, amount)?;

        self.withdraw_tokens(!is_x, amount_out)?;

        Ok(())
    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.user_x.to_account_info(),
                self.vault_x.to_account_info(),
            ),
            false => (
                self.user_y.to_account_info(),
                self.vault_y.to_account_info(),
            ),
        };

        transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from,
                    to,
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
        )?;

        Ok(())
    }

    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
            false => (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            ),
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];

        transfer(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Transfer {
                    from,
                    to,
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        Ok(())
    }
}
