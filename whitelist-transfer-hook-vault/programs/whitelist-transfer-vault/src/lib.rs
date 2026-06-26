pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

use spl_discriminator::SplDiscriminate;

declare_id!("4Vw9HhrxZ3nUV9WvmymJBm35kUcVpvffz71D6BgwEEZR");

/// Whitelist Transfer Vault Program
///
/// This program implements a vault system with transfer hooks that enforce
/// whitelist-based access control on token transfers. Only whitelisted addresses
/// can deposit, withdraw, or transfer tokens managed by this vault.
#[program]
pub mod whitelist_transfer_vault {

    use super::*;

    /// Initializes the vault configuration, mint, and associated token account.
    /// Also creates the vault's own whitelist entry.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(ctx.bumps)
    }

    /// Mints new tokens to a user's associated token account.
    /// Only the vault admin can call this instruction.
    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        ctx.accounts.mint_token(amount)
    }

    /// Adds a user to the whitelist, allowing them to interact with the vault.
    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>, user: Pubkey) -> Result<()> {
        ctx.accounts.add_to_whitelist(ctx.bumps, user)
    }

    /// Removes a user from the whitelist and closes their whitelist account.
    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>, user: Pubkey) -> Result<()> {
        ctx.accounts.remove_from_whitelist(user)
    }

    /// Deposits tokens from a user's account into the vault.
    /// Requires the user to be whitelisted.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    /// Withdraws tokens from the vault to a user's account.
    /// Requires the user to be whitelisted.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    /// Executes the transfer hook logic during token transfers.
    /// Validates that both source and destination are whitelisted.
    #[instruction(
        discriminator = spl_transfer_hook_interface::instruction::ExecuteInstruction::SPL_DISCRIMINATOR_SLICE
    )]
    pub fn execute(ctx: Context<Execute>, amount: u64) -> Result<()> {
        transfer_hook::execute_hook(ctx, amount)
    }
}
