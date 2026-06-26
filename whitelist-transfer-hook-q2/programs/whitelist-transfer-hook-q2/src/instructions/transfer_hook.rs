use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount, BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{Mint, TokenAccount},
};

use crate::state::WhitelistEntry;

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint,
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner    can be a system account or a PDA owned by
    /// another program. Validation is done implicitly: if the WhitelistEntry PDA for
    /// this owner does not exist, the seeds constraint below will fail, blocking the
    /// transfer.
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList    seed-validated, written by initialize_transfer_hook.
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    // The entry PDA is derived from the owner key passed in. If this account does not
    // exist on-chain the constraint fails and the transfer is rejected    no explicit
    // `.contains()` check needed, the runtime does it for us.
    #[account(
        seeds = [b"whitelist", owner.key().as_ref()],
        bump = entry.bump,
    )]
    pub entry: Account<'info, WhitelistEntry>,
}

impl<'info> TransferHook<'info> {
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        self.check_is_transferring()?;

        // Reaching this point means the entry PDA was resolved successfully,
        // so the owner is whitelisted. Log for observability.
        msg!(
            "Transfer allowed    owner {} is whitelisted",
            self.source_token.owner
        );

        Ok(())
    }

    fn check_is_transferring(&mut self) -> Result<()> {
        let source_token_info = self.source_token.to_account_info();
        let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
        let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

        if !bool::from(account_extension.transferring) {
            return Err(ProgramError::InvalidAccountData.into());
        }

        Ok(())
    }
}