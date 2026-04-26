use anchor_lang::prelude::*;

declare_id!("CpS2i2xGvPwdhCxEwHhWwHQcGD3npT5awo8ouNerm7P9");

#[program]
pub mod assignment0 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
