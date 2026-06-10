use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use mpl_core::accounts::BaseCollectionV1;

use crate::error::ErrorCode;
use crate::state::Config;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
        seeds = [b"config", collection.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    // Ensures only the collection owner can initialize rewards for it
    #[account(has_one = update_authority @ ErrorCode::InvalidUpdateAuthority)]
    pub collection: Account<'info, BaseCollectionV1>,

    /// CHECK: PDA derived from collection address, verified by seeds constraint; used for collection operations
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    // Creates a new SPL token mint for rewards, owned by the config PDA
    #[account(
        init,
        payer = admin,
        mint::decimals = 6,
        mint::authority = config,
        seeds = [b"rewards_mint", config.key().as_ref()],
        bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Initialize>, rewards_bps: u16, freeze_period: u16) -> Result<()> {
    ctx.accounts.config.set_inner(Config {
        rewards_bps,
        freeze_period,
        rewards_bump: ctx.bumps.rewards_mint,
        bump: ctx.bumps.config,
    });

    Ok(())
}
