use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    seeds::Seed,
    state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

use crate::{VAULT_PROGRAM_ID, WHITELIST_SEED};

/// Accounts required to initialize the extra account meta list for a mint.
#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    /// Payer for the extra account meta list account.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The mint to configure extra accounts for.
    pub mint: InterfaceAccount<'info, Mint>,

    /// The extra account meta list account (TLV format).
    /// CHECK: Transfer-hook validation account stores TLV extra-account metadata.
    #[account(
        init,
        payer = payer,
        space = ExtraAccountMetaList::size_of(3).unwrap(),
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Initializes the extra account meta list with the accounts required by the transfer hook.
/// This tells Token-2022 which additional accounts to include for every transfer of this mint.
pub fn initialize_extra_account_meta_list(
    ctx: Context<InitializeExtraAccountMetaList>,
) -> Result<()> {
    // ExtraAccountMetaList is the "account recipe" Token-2022 reads before calling the hook.
    // It tells Token-2022 which extra accounts to include for every transfer of this mint.
    let account_metas = vec![
        // Account index 5 in the hook call will be the vault program id.
        ExtraAccountMeta::new_with_pubkey(&VAULT_PROGRAM_ID, false, false)?,

        // Resolve source owner whitelist PDA:
        // ["whitelist", source_token_account.owner], derived under the vault program id.
        ExtraAccountMeta::new_external_pda_with_seeds(
            5,
            &[
                Seed::Literal {
                    bytes: WHITELIST_SEED.to_vec(),
                },
                Seed::AccountData {
                    account_index: 0,
                    data_index: 32,
                    length: 32,
                },
            ],
            false,
            false,
        )?,

        // Resolve destination owner whitelist PDA:
        // ["whitelist", destination_token_account.owner], also under the vault program id.
        ExtraAccountMeta::new_external_pda_with_seeds(
            5,
            &[
                Seed::Literal {
                    bytes: WHITELIST_SEED.to_vec(),
                },
                Seed::AccountData {
                    account_index: 2,
                    data_index: 32,
                    length: 32,
                },
            ],
            false,
            false,
        )?,
    ];

    let mut data = ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?;
    ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, &account_metas)?;

    Ok(())
}