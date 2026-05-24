use crate::state::UserAccount;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CallbackRandomness<'info> {
    /// VRF program identity must be signer — proves oracle called this
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,
    /// The user account we update with random data
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

impl<'info> CallbackRandomness<'info> {
    pub fn callback_randomness(&mut self, randomness: [u8; 32]) -> Result<()> {
        let random_u64 = u64::from_le_bytes(randomness[..8].try_into().unwrap());

        msg!("VRF randomness received: {:?}", randomness);
        msg!("Storing as u64: {}", random_u64);

        self.user_account.data = random_u64;

        Ok(())
    }
}
