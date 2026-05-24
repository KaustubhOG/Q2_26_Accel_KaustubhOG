use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::delegate;
use ephemeral_rollups_sdk::cpi::{delegate_account, DelegateAccounts, DelegateConfig};

#[delegate]
#[derive(Accounts)]
pub struct Delegate<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: The account to delegate
    #[account(
        mut,
        del,
        seeds = [b"user", user.key().as_ref()],
        bump,
    )]
    pub user_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Delegate<'info> {
    pub fn delegate(&mut self) -> Result<()> {
        let pda_seeds: &[&[u8]] = &[b"user", self.user.key.as_ref()];

        delegate_account(
            DelegateAccounts {
                payer: &self.user.to_account_info(),
                pda: &self.user_account,
                owner_program: &self.owner_program,
                buffer: &self.buffer_user_account,
                delegation_record: &self.delegation_record_user_account,
                delegation_metadata: &self.delegation_metadata_user_account,
                delegation_program: &self.delegation_program,
                system_program: &self.system_program.to_account_info(),
            },
            pda_seeds,
            DelegateConfig::default(),
        )?;

        Ok(())
    }
}
