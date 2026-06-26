use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke_signed},
};
use spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi;

/// Account infos required for a token transfer with hook.
pub struct TokenTransferWithHookAccounts<'a, 'info> {
    pub source: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub extra_meta_account_list: AccountInfo<'info>,
    pub vault_program: AccountInfo<'info>,
    pub source_whitelist: AccountInfo<'info>,
    pub destination_whitelist: AccountInfo<'info>,
    pub hook_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub signer_seeds: &'a [&'a [&'a [u8]]],
}

/// Performs a transfer_checked with the transfer hook program.
/// Appends extra accounts required by the hook and invokes the instruction.
pub fn transfer_checked_with_hook<'a, 'info>(
    accounts: TokenTransferWithHookAccounts<'a, 'info>,
    amount: u64,
    decimals: u8,
) -> Result<()> {
    let mut ix = anchor_spl::token_interface::spl_token_2022::instruction::transfer_checked(
        accounts.token_program.key,
        accounts.source.key,
        accounts.mint.key,
        accounts.destination.key,
        accounts.authority.key,
        &[],
        amount,
        decimals,
    )?;

    let mut account_infos = vec![
        accounts.source.clone(),
        accounts.mint.clone(),
        accounts.destination.clone(),
        accounts.authority.clone(),
    ];

    let hook_program_id = *accounts.hook_program.key;

    // Extra accounts required by the transfer hook:
    // 1. extra_account_meta_list - TLV account for hook validation
    // 2. vault_program - used to derive whitelist PDAs
    // 3. source_whitelist - source account's whitelist entry
    // 4. destination_whitelist - destination account's whitelist entry
    // 5. hook_program - the transfer hook program itself
    let additional_accounts = vec![
        accounts.extra_meta_account_list.clone(),
        accounts.vault_program,
        accounts.source_whitelist,
        accounts.destination_whitelist,
        accounts.hook_program,
    ];

    add_extra_accounts_for_execute_cpi(
        &mut ix,
        &mut account_infos,
        &hook_program_id,
        accounts.source,
        accounts.mint,
        accounts.destination,
        accounts.authority,
        amount,
        &additional_accounts,
    )?;

    account_infos.push(accounts.token_program);

    invoke_signed(
        &Instruction {
            program_id: ix.program_id,
            accounts: ix.accounts,
            data: ix.data,
        },
        &account_infos,
        accounts.signer_seeds,
    )?;

    Ok(())
}
