use crate::{state::*, VAULT_CONFIG_SEED};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{mint_to, MintTo},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

/// Accounts required to mint tokens to a user.
#[derive(Accounts)]
pub struct MintToken<'info> {
    /// Admin authority (must match vault admin).
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Recipient of the minted tokens (authority of the ATA).
    /// CHECK: The recipient authority for the associated token account.
    pub user: UncheckedAccount<'info>,

    /// Vault config PDA (authority for minting).
    #[account(
        seeds = [VAULT_CONFIG_SEED],
        bump = vault_pda.bump,
    )]
    pub vault_pda: Account<'info, VaultConfig>,

    /// The mint to mint tokens from.
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    /// User's associated token account (created if needed).
    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> MintToken<'info> {
    /// Mints `amount` tokens to the user's ATA using vault PDA as authority.
    pub fn mint_token(&mut self, amount: u64) -> Result<()> {
        let signer_seeds: &[&[&[u8]]] = &[&[VAULT_CONFIG_SEED, &[self.vault_pda.bump]]];
        mint_to(
            CpiContext::new_with_signer(
                self.token_program.key(),
                MintTo {
                    mint: self.mint.to_account_info(),
                    to: self.user_token_account.to_account_info(),
                    authority: self.vault_pda.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        msg!(
            "Minted {} tokens to {} ATA {}",
            self.user_token_account.amount,
            self.user.key(),
            self.user_token_account.key()
        );
        Ok(())
    }
}
