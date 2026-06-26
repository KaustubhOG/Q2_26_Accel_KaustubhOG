pub mod store;
pub mod errors;
pub mod config;
pub mod handlers;

use anchor_lang::prelude::*;
use spl_discriminator::SplDiscriminate;

pub use store::*;
pub use config::*;
pub use handlers::*;

/// Program ID of the vault program that owns whitelist accounts.
pub const VAULT_PROGRAM_ID: Pubkey = pubkey!("4Vw9HhrxZ3nUV9WvmymJBm35kUcVpvffz71D6BgwEEZR");

/// Seed used to derive whitelist entry PDAs.
pub const WHITELIST_SEED: &[u8] = b"whitelist-entry";

/// Seed used to derive vault config PDAs.
pub const VAULT_CONFIG_SEED: &[u8] = b"vault-config";

declare_id!("GKT2mM8YqWFiYSuNsyNPVzSfaZBRwGnosX3eUJt2UKQS");

/// Transfer Hook Program
///
/// This program implements the transfer hook logic for the whitelist vault.
/// It validates that both source and destination accounts are whitelisted
/// before allowing token transfers.
#[program]
pub mod transfer_hook {
    use super::*;

    /// Initializes the extra account meta list for the mint.
    /// This tells Token-2022 which additional accounts to include during transfers.
    pub fn initialize_extra_account_metas(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        meta_setup::initialize_extra_account_meta_list(ctx)
    }

    /// Executes the transfer hook validation during token transfers.
    /// Verifies that both source and destination are whitelisted.
    #[instruction(
        discriminator = spl_transfer_hook_interface::instruction::ExecuteInstruction::SPL_DISCRIMINATOR_SLICE
    )]
    pub fn transfer_hook(ctx: Context<Execute>, amount: u64) -> Result<()> {
        hook_runner::execute(ctx, amount)
    }
}