use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token_interface::{Mint, TokenInterface, TokenAccount, TransferChecked, transfer_checked}
};


#[derive(Accounts)]
#[instruction(seeds: u64)]
pub struct Create<'info> {
    
    #[account(mut)]
    pub creator: Signer<'info>,


    // The mint accounts for the two tokens involved in the escrow
    #[account(
        mint::token_program = token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
        mint::token_program = token_program
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,


    // creates an associated token account for the creator to deposit tokens into the vault, only creator can withdraw it
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = creator,
        associated_token::token_program = token_program,
    )]
    pub creator_ata_a: InterfaceAccount<'info, TokenAccount>,


    // creates an escrow account to store the details of the escrow.
    #[account(
        init,
        payer = creator,
        seeds = [b"escrow", creator.key().as_ref(), seeds.to_le_bytes().as_ref()],
        space = Escrow::DISCRIMINATOR.len() + Escrow::INIT_SPACE,
        bump,
    )]
    pub escrow: Account<'info, Escrow>,

    // creates a vault account to hold the tokens deposited by the creator, the vault is owned by the escrow account
    #[account(
        init,
        payer = creator,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

impl<'info> Create<'info> {
    //Initialize the escrow
    pub fn init_escrow(&mut self, seed: u64, receive: u64, bumps: &CreateBumps) -> Result<()> {

        self.escrow.set_inner(Escrow {          //set_inner writes escrow data to blockchain
            seed, 
            creator: self.creator.key(), 
            mint_a: self.mint_a.key(), 
            mint_b: self.mint_b.key(), 
            receive: receive, 
            bump:  bumps.escrow
        });

        Ok(())
    }

    //Deposit tokens from creator to vault
    pub fn deposit(&mut self, deposit: u64) ->Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.creator_ata_a.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.creator.to_account_info()
        };

        let cpi_ctx = CpiContext::new(
            self.token_program.key(), 
            transfer_accounts
        );

        // A CPI call has been made to transfer_checked function to transfer tokens
        // from creators wallet to vault
        transfer_checked(
            cpi_ctx, 
            deposit, 
            self.mint_a.decimals
        )?;

        Ok(())
    }
}