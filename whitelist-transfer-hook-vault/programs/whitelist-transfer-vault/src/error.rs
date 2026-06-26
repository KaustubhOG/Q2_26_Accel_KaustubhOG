use anchor_lang::prelude::*;

/// Errors that can occur within the vault program.
#[error_code]
pub enum VaultError {
    /// The signer is not authorized to perform this action.
    #[msg("You are not authorized to perform this action")]
    Unauthorized,
    /// The vault's mint has not been initialized yet.
    #[msg("Mint has not been initialized yet")]
    MintNotInitialized,
    /// The user is not on the whitelist and cannot perform the operation.
    #[msg("User is not whitelisted")]
    NotWhitelisted,
    /// The transfer hook was invoked outside of a token transfer context.
    #[msg("Transfer hook invoked outside a transferring context")]
    NotTransferring,
}
