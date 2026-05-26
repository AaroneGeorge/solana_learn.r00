use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer},
};
use constant_product_curve::ConstantProduct; // AMM math library for the curve: x*y=k

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Box<Account<'info, Config>>,
    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Box<Account<'info, Mint>>,
    #[account(
        mut,
        address = config.mint_x,
    )]
    pub mint_x: Box<Account<'info, Mint>>,
    #[account(
        mut,
        address = config.mint_y,
    )]
    pub mint_y: Box<Account<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(
        &mut self,
        amount: u64, // Amount of LP tokens that the user wants to "claim"
        max_x: u64,  // Maximum amount of token X that the user is willing to deposit
        max_y: u64,  // Maximum amount of token Y that the user is willing to deposit
    ) -> Result<()> {
        require!(self.config.locked == false, AmmError::PoolLocked);
        require!(amount != 0, AmmError::InvalidAmount);


        // Logic: if no LP tokens issue yet, if no token X,Y in pool yet, pool is empty,
        // do first deposit, use max x,y directly, user sets the initial price ration
        // ex. 100USDC and 50SOL sets ratio as 2 USDC per SOL

        // else: means pool has liquidity, do subsequent deposits,
        // use ConstantProduct formula to calculate exact amounts
        // Calculate: How much USDC (x) and SOL (y) needed to maintain the price ratio?
        let (x, y) = match self.mint_lp.supply == 0
            && self.vault_x.amount == 0
            && self.vault_y.amount == 0
        {
            true => (max_x, max_y),
            false => {
                let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                    self.vault_x.amount,
                    self.vault_y.amount,
                    self.mint_lp.supply,
                    amount,
                    6,
                )
                .unwrap();
                (amounts.x, amounts.y)
            }
        };

        // Slippage check: if x>max x or y > max y, user is trying to deposit too much 
        // and their price is worse than they expected, so we should error out to protect them
        // prevents Front running
        require!(x <= max_x && y <= max_y, AmmError::SlippageExceeded);

        // deposit token x
        self.deposit_tokens(true, x)?;
        // deposit token y
        self.deposit_tokens(false, y)?;
        // mint lp tokens
        self.mint_lp_tokens(amount)
    }

    pub fn deposit_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
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

    pub fn mint_lp_tokens(&self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = MintTo {
            mint: self.mint_lp.to_account_info(),
            to: self.user_lp.to_account_info(),
            authority: self.config.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];

        let ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        mint_to(ctx, amount)
    }
}


//   ConstantProduct::xy_deposit_amounts_from_l EQUATION
//   amount_x = (amount_lp * vault_x) / total_supply
//   amount_y = (amount_lp * vault_y) / total_supply


// Deposit Flow
// 
//   1. User calls deposit(1000, 100, 50) (want 1000 LP, willing to spend up to
//   100 USDC and 50 SOL)
//   2. Validation: Pool not locked, amount > 0
//   3. Calculate: If pool empty, use max values; else use ConstantProduct
//   formula
//   4. Slippage check: Ensure calculated amounts ≤ max values
//   5. Transfer USDC: User signs to authorize USDC transfer to vault_x
//   6. Transfer SOL: User signs to authorize SOL transfer to vault_y
//   7. Mint LP: Config (PDA) signs to mint 1000 LP tokens into user_lp account
//   8. User now owns LP tokens representing their share of the pool