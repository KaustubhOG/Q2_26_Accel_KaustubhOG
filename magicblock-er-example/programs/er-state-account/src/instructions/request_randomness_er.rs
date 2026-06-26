use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::commit;
use ephemeral_vrf_sdk::anchor::vrf;
use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
use ephemeral_vrf_sdk::types::SerializableAccountMeta;

use crate::state::UserAccount;

// #[commit] makes magic_context and magic_program available for committing
#[commit]
#[vrf]
#[derive(Accounts)]
pub struct RequestRandomnessEr<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,
    /// CHECK: Oracle queue   validated by the VRF program itself
    #[account(mut)]
    pub oracle_queue: AccountInfo<'info>,
}

impl<'info> RequestRandomnessEr<'info> {
    pub fn request_randomness_er(&self, client_seed: u8) -> Result<()> {
        let ix = create_request_randomness_ix(RequestRandomnessParams {
            payer: self.user.key(),
            oracle_queue: self.oracle_queue.key(),
            callback_program_id: crate::ID,
            // Points to our ER callback instruction
            callback_discriminator: crate::instruction::CallbackRandomnessEr::DISCRIMINATOR
                .to_vec(),
            caller_seed: [client_seed; 32],
            accounts_metas: Some(vec![SerializableAccountMeta {
                pubkey: self.user_account.key(),
                is_signer: false,
                is_writable: true,
            }]),
            ..Default::default()
        });

        self.invoke_signed_vrf(&self.user.to_account_info(), &ix)?;

        Ok(())
    }
}
