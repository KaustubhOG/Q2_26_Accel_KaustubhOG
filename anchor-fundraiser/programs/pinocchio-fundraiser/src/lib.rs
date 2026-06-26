pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use pinocchio::{
    account::AccountView,
    address::Address,
    entrypoint,
    error::ProgramError,
    ProgramResult,
};

entrypoint!(process_instruction);

pub static ID: Address = Address::new_from_array([
    0xE0, 0x1D, 0xF1, 0xD6, 0x72, 0x9E, 0x1A, 0x6D,
    0x1A, 0x42, 0x38, 0x25, 0x0E, 0xE1, 0x39, 0xC1,
    0x58, 0x8F, 0x9B, 0xA2, 0xA3, 0x5D, 0xC3, 0xFE,
    0x89, 0x53, 0x6A, 0x1C, 0x48, 0xC8, 0xF5, 0xBB,
]);

pub fn process_instruction(
    _program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.split_first() {
        // Initialize(amount: u64, duration: u8)
        Some((0, data)) => {
            if data.len() < 9 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
            let duration = data[8];
            instructions::Initialize::process(&ID, accounts, amount, duration)
        }
        // Contribute(amount: u64)
        Some((1, data)) => {
            if data.len() < 8 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
            instructions::Contribute::process(&ID, accounts, amount)
        }
        // CheckContributions
        Some((2, _)) => instructions::CheckContributions::process(&ID, accounts),
        // Refund
        Some((3, _)) => instructions::Refund::process(&ID, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
