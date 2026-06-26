#![allow(unexpected_cfgs)]
#![allow(deprecated)]

pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

use anchor_lang::prelude::*;
use spl_discriminator::SplDiscriminate;
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

declare_id!("BEz1qvQf6FABo21oFQ7DMJVPcYhrK2MR9yoFpepHd1Jf");

#[program]
pub mod whitelist_transfer_hook_q2 {
    use super::*;

    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>, address: Pubkey) -> Result<()> {
        ctx.accounts.add_to_whitelist(&ctx.bumps)
    }

    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>, address: Pubkey) -> Result<()> {
        ctx.accounts.remove_from_whitelist()
    }

    pub fn initialize_transfer_hook(ctx: Context<InitializeExtraAccountMetaList>) -> Result<()> {
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas,
        )
        .unwrap();

        Ok(())
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        ctx.accounts.transfer_hook(amount)
    }
}