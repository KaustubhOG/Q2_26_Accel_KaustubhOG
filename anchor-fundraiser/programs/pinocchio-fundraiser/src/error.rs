use pinocchio::error::ProgramError;

#[repr(u32)]
pub enum FundraiserError {
    TargetNotMet = 6000,
    TargetMet = 6001,
    ContributionTooBig = 6002,
    ContributionTooSmall = 6003,
    MaximumContributionsReached = 6004,
    FundraiserNotEnded = 6005,
    FundraiserEnded = 6006,
    InvalidAmount = 6007,
}

impl From<FundraiserError> for ProgramError {
    fn from(e: FundraiserError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
