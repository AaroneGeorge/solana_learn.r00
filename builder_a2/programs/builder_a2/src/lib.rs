use anchor_lang::prelude::*;

declare_id!("EsfzrVNn3fuSacJw55y1AamEa9NCPudV3hzEM8K7Kf7i");

#[program]
pub mod builder_a2 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
