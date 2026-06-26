use anchor_lang::prelude::*;

/// Program ID of the transfer hook program that validates whitelist entries.
pub const TRANSFER_HOOK_PROGRAM_ID: Pubkey =
    pubkey!("GKT2mM8YqWFiYSuNsyNPVzSfaZBRwGnosX3eUJt2UKQS");

/// Seed used to derive whitelist entry PDAs.
pub const WHITELIST_SEED: &[u8] = b"whitelist-entry";

/// Seed used to derive the vault config PDA.
pub const VAULT_CONFIG_SEED: &[u8] = b"vault-config";

/// Seed used to derive the mint PDA.
pub const MINT_SEED: &[u8] = b"mint-seed";
