use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::commit;
use ephemeral_rollups_sdk::ephem::commit_accounts;

use crate::state::UserAccount;

// #[commit] needed so we can commit the updated state back to mainnet
#[commit]
#[derive(Accounts)]
pub struct CallbackRandomnessEr<'info> {
    /// VRF program identity must be signer — proves oracle called this
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,
    /// The user account we update with random data
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    /// Payer needed for commit
    #[account(mut)]
    pub payer: Signer<'info>,
}

impl<'info> CallbackRandomnessEr<'info> {
    pub fn callback_randomness_er(&mut self, randomness: [u8; 32]) -> Result<()> {
        let random_u64 = u64::from_le_bytes(randomness[..8].try_into().unwrap());

        msg!("ER VRF randomness received: {:?}", randomness);
        msg!("Storing as u64: {}", random_u64);

        // Update state with random value
        self.user_account.data = random_u64;

        // Commit the updated state back to base chain immediately
        commit_accounts(
            &self.payer.to_account_info(),
            vec![&self.user_account.to_account_info()],
            &self.magic_context,
            &self.magic_program,
        )?;

        Ok(())
    }
}
