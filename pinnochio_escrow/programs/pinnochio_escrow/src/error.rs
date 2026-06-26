use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscrowError {
    InvalidInstructionData = 0,
    NotEnoughAccountKeys = 1,
    MissingRequiredSignature = 2,
    IncorrectProgramId = 3,
    AccountAlreadyInitialized = 4,
    InsufficientFunds = 5,
    InvalidAccountData = 6,
    IllegalOwner = 7,
    EscrowExpired = 8,
    UnauthorizedRefund = 9,
    InvalidVault = 10,
}

impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub fn log_error(error: &EscrowError) {
    let msg = match error {
        EscrowError::InvalidInstructionData => "InvalidInstructionData",
        EscrowError::NotEnoughAccountKeys => "NotEnoughAccountKeys",
        EscrowError::MissingRequiredSignature => "MissingRequiredSignature",
        EscrowError::IncorrectProgramId => "IncorrectProgramId",
        EscrowError::AccountAlreadyInitialized => "AccountAlreadyInitialized",
        EscrowError::InsufficientFunds => "InsufficientFunds",
        EscrowError::InvalidAccountData => "InvalidAccountData",
        EscrowError::IllegalOwner => "IllegalOwner",
        EscrowError::EscrowExpired => "EscrowExpired",
        EscrowError::UnauthorizedRefund => "UnauthorizedRefund",
        EscrowError::InvalidVault => "InvalidVault",
    };
    pinocchio_log::log!("ESCROW_ERROR: {}", msg);
}
