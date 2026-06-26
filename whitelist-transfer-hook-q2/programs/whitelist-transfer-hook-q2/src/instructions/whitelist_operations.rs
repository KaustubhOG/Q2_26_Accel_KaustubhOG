use anchor_lang::prelude::*;

use crate::state::WhitelistEntry;

#[derive(Accounts)]
#[instruction(address: Pubkey)]
pub struct AddToWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    // One entry PDA per address. init_if_needed is intentionally NOT used    double-adding
    // the same address should be an explicit no-op caught at the client level, not silently
    // swallowed. Using `init` means the runtime errors on duplicate, which is the right
    // behaviour for an access-control list.
    #[account(
        init,
        payer = admin,
        space = WhitelistEntry::LEN,
        seeds = [b"whitelist", address.as_ref()],
        bump,
    )]
    pub entry: Account<'info, WhitelistEntry>,

    pub system_program: Program<'info, System>,
}

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, bumps: &AddToWhitelistBumps) -> Result<()> {
        self.entry.bump = bumps.entry;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(address: Pubkey)]
pub struct RemoveFromWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    // close = admin returns the rent lamports to the admin and zeroes the account,
    // which is equivalent to removing the address from the whitelist.
    #[account(
        mut,
        close = admin,
        seeds = [b"whitelist", address.as_ref()],
        bump = entry.bump,
    )]
    pub entry: Account<'info, WhitelistEntry>,

    pub system_program: Program<'info, System>,
}

impl<'info> RemoveFromWhitelist<'info> {
    pub fn remove_from_whitelist(&mut self) -> Result<()> {
        // The `close` constraint handles lamport transfer and account zeroing.
        // Nothing extra to do here; the constraint does the work.
        Ok(())
    }
}