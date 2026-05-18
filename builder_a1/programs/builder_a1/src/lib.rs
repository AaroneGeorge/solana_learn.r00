use anchor_lang::prelude::*;

// Declares a module and includes it in crate
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

// Re-exports everything from the modules for access.
pub use constants::*;
pub use error::*;
pub use instructions::*;
pub use state::*;


declare_id!("BBpEqqsvd4mTfs4wJbnHTMx4jD3v5mvHeBaRphd2gGaC");

#[program]
pub mod builder_a1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()
    }
}



//   Import paths — going back directories:
//   // Same directory
//   use crate::state::VaultState;

//   // Parent directory (one level up)
//   use super::state::VaultState;
  
//   // Two levels up
//   use super::super::state::VaultState;

//   // From root of crate
//   use crate::error::ErrorCode;