use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

// Mint: token definition, used to reference which token we are working with
// Token: Token program, used to perform create accounts, transfer, mint, burn, etc
// TokenAccount: An account that holds a balance of a specific token, associated with a mint

use crate::state::Config;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    // Mint account for the LP token, owned by the config account
    // The reciept token that users get when they provide liquidity, represents their share of the pool
    // TLDR; LP token definition, tracks thhe pool's receipt token.
    #[account(
        init,
        payer = initializer,
        seeds = [b"lp", config.key.as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Account<'info, Mint>,

    // ATA's for token X and Y, owned by the config account
    // vault_x, vault_y holds token X and Y deposits respec.,
    // when users swap, tokens move between vaults while prices adjust
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    // Config account, holds the pool's brain which stores fee, locked status, token references, etc
    #[account(
        init,
        payer = initializer,
        seeds = [b"config", seed.to_le_bytes().as_ref()],
        bump,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
    )]
    pub config: Account<'info, Config>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn init(
        &mut self,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
        bumps: InitializeBumps,
    ) -> Result<()> {
        self.config.set_inner(Config { // set_inner serializes and writes config struct to the config account.
            seed,
            authority,
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            fee,
            locked: false,
            config_bump: bumps.config,
            lp_bump: bumps.mint_lp,
        });

        Ok(())
    }
}

//   #[derive(Accounts)] = Anchor macro that auto generates account validation logic
//   - What it does:
//     a. Validates that all accounts are present in the transaction
//     b. Deserializes account data into Rust types
//     c. Enforces constraints (like mutability, signer status)
//     d. Returns a helpful error if an account is missing or invalid

// Why this PDA scheme for config account?
// seed.to_le_bytes().as_ref()
//   - Different pools can have the same token pair (mint_x, mint_y)
//   - The seed parameter distinguishes them
//   - Example:
// - Pool 1: seed = 0, has USDC + SOL
// - Pool 2: seed = 1, has USDC + SOL (same tokens, different pool!)
// - Addresses are different because seeds differ

// Accounts used in thsi instruction:
// Category A: Token Definitions: these define what tokens exist and their properties
//   ┌─────────┬────────────────────────┬───────────────────────────────────┐
//   │ Account │         Owner          │              Purpose              │
//   ├─────────┼────────────────────────┼───────────────────────────────────┤
//   │ mint_x  │ Token Program          │ Token X definition (e.g., USDC,   │
//   │         │                        │ decimals, supply)                 │
//   ├─────────┼────────────────────────┼───────────────────────────────────┤
//   │ mint_y  │ Token Program          │ Token Y definition (e.g., SOL,    │
//   │         │                        │ decimals, supply)                 │
//   ├─────────┼────────────────────────┼───────────────────────────────────┤
//   │ mint_lp │ Token Program          │ LP token definition — tracks the  │
//   │         │ (authority = config)   │ pool's receipt token              │
//   └─────────┴────────────────────────┴───────────────────────────────────┘
// Category B: Token Accounts: these hold actual token balances and are associated with a mint
//   ┌─────────┬───────────────┬────────┬───────────────────────────┐
//   │ Account │ What it holds │ Owner  │          Purpose          │
//   ├─────────┼───────────────┼────────┼───────────────────────────┤
//   │ vault_x │ USDC tokens   │ config │ Pool's reserve of Token X │
//   ├─────────┼───────────────┼────────┼───────────────────────────┤
//   │ vault_y │ SOL tokens    │ config │ Pool's reserve of Token Y │
//   └─────────┴───────────────┴────────┴───────────────────────────┘
//   Category C: Configuration & Authority

//   ┌─────────┬───────────────────────────────────────────────────────┐
//   │ Account │                        Purpose                        │
//   ├─────────┼───────────────────────────────────────────────────────┤
//   │ config  │ NOT just authorization — it's the pool's state/memory │
//   └─────────┴───────────────────────────────────────────────────────┘
//     - It's the pool's brain — stores fee, locked status, token references
//     - It's the authority owner of vault_x, vault_y, and mint_lp
//     - When someone swaps, config signs to move tokens between vaults
//     - When someone deposits, config signs to mint LP tokens
