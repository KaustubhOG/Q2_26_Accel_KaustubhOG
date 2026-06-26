use anchor_lang::prelude::*;

declare_id!("FRtqqYN6nkehBgSADyWWoyv5J6yv1F96yX9kj2W7Z5EE");

#[program]
pub mod mplstacking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
