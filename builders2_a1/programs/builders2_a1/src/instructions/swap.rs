use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{errors::AmmError, state::Config};

// TLDR; Exchange one token for another, using constant product formula with fees
#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump
    )]
    pub config: Account<'info, Config>,

    // Not used in swap, just here for curve initialization to get the fee and supply
    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config
    )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user
    )]
    pub user_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user 
    )]
    pub user_y: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>
}

impl<'info> Swap<'info> {
    // Swap logic: buying token Y, selling token X, then set is_x to true
    // buying token X, selling token Y, then set is_x to false
    // min is used for slippage protection, if output < min, reject trade.
    pub fn swap(&mut self, is_x: bool, amount: u64, min: u64) -> Result<()> {
    
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            self.config.fee, // fee is in basis points, so 30 means 0.3% fee, which goes to liquidity providers as incentive
            None
        ).map_err(AmmError::from)?;

        //   enum LiquidityPair {
        //   X,  // Selling Token X (buying Token Y)
        //   Y,  // Selling Token Y (buying Token X)
        //   }   

        let res = match is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y
        };

        let swaps = curve.swap(res, amount, min).map_err(AmmError::from)?;

        // the formula curve.swap() calculates;
        //   Formula (with fees):
        //     Input amount × (1 - fee) = amount_after_fee
        //     New_X = old_X + amount_after_fee (if selling X)
        //     New_Y = old_Y + new_amount_out_Y

        //     Must satisfy: New_X × New_Y ≥ old_X × old_Y
        //     output = old_Y - (old_X × old_Y / New_X)

        require!(swaps.deposit != 0, AmmError::InvalidAmount);

        self.deposit_tokens(is_x, swaps.deposit)?;
        self.withdraw_tokens(!is_x, swaps.withdraw)?;
        Ok(())
    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
    
        let (from, to) = match is_x {
            true => (
                self.user_x.to_account_info(),
                self.vault_x.to_account_info(),
            ),
            false => (
                self.user_y.to_account_info(),
                self.vault_y.to_account_info(),
            ),
        };

        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.user.to_account_info(),
        };

        let ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(ctx, amount)
    }

    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        
        let (from, to) = match is_x {
            true => (&self.vault_x, &self.user_x),
            false => (&self.vault_y, &self.user_y),
        };

        let cpi_accounts = Transfer {
            from: from.to_account_info(),
            to: to.to_account_info(),
            authority: self.config.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        transfer(cpi_ctx, amount)?;
        Ok(())
    }
}


// A note on &mut self and &self in the swap, deposit, withdraw functions:
//   ┌───────────┬──────────────────┬────────────────────────────┐
//   │ Signature │  What it allows  │          Best for          │
//   ├───────────┼──────────────────┼────────────────────────────┤
//   │ &self     │ Read-only access │ Methods that don't change  │
//   │           │                  │ state                      │
//   ├───────────┼──────────────────┼────────────────────────────┤
//   │ &mut self │ Read + write     │ Methods that need to       │
//   │           │ access           │ change state               │
//   └───────────┴──────────────────┴────────────────────────────┘

// In deposit.rs and withdraw.rs, deposit_tokens and withdraw_tokens function uses &self 
// because they only read from the accounts to do token transfers, 
// they don't need to change any state in the accounts(Deposit or Withdraw Struct).

// In swap.rs, swap function uses &mut self because it might update the Swap struct
// in current code, it doesnt update any state in Swap struct