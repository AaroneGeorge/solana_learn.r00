use anchor_lang::prelude::*;
use crate::state::Escrow;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked, CloseAccount, close_account}
};


#[derive(Accounts)]
pub struct Accept<'info> {
    #[account(mut)]
    pub acceptor: Signer<'info>,

    #[account(mut)]
    pub creator: SystemAccount<'info>,

    #[account(
        mint::token_program = token_program
    )]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mint::token_program = token_program
    )]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,


    // this is where acceptor receives token A from the vault
    #[account(
        init_if_needed, // create ATA only if it doesnt exist, if it already exists, just use it
        payer = acceptor,
        associated_token::mint = mint_a,
        associated_token::authority = acceptor,
        associated_token::token_program = token_program
    )]
    pub acceptor_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,


    // the acceptor must have token B to pay the creator, acceptor sends token B to creator's ATA
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = acceptor,
        associated_token::token_program = token_program,
    )]
    pub acceptor_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // this is where creator receives token B from acceptor.
    #[account(
        init_if_needed,
        payer = acceptor,
        associated_token::mint = mint_b,
        associated_token::authority = creator,
        associated_token::token_program = token_program,
    )]
    pub creator_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,


    // once escrow is accepted, the escrow account is closed and the vault is closed
    // the rent SOL is refunded to the creator
    #[account(
        mut,
        close = creator, // close function closes this account and return its rent to creator
        seeds = [b"escrow", escrow.creator.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
        // below has_one verifies that the escrow account is associated with the correct mint and creator, 
        // it checks that the mint_a, mint_b and creator fields in the escrow account 
        // match the accounts passed in this context

        // REASON: Prevents someone from passing in the wrong mint on creator and stealing funds
        has_one = mint_a,
        has_one = mint_b,
        has_one = creator,
    )]
    pub escrow: Account<'info, Escrow>,

    // Token A is being withdrawn from this vault.
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>
}

impl<'info> Accept<'info> {
    //Transfer token B from acceptor to creator
    pub fn deposit(&mut self) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from: self.acceptor_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.creator_ata_b.to_account_info(),
            authority: self.acceptor.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            cpi_accounts
        );

        transfer_checked(cpi_ctx, self.escrow.receive, self.mint_b.decimals)
    }

    //Withdraw token A from the vault to acceptor and close the vault
    pub fn withdraw_and_close_vault(&mut self) -> Result<()> {
        
        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.acceptor_ata_a.to_account_info(),
            authority: self.escrow.to_account_info()
        };

        // the signer seeds are reconstructed to prove that the program has the authority to transfer tokens from the vault,
        // since the vault is owned by the escrow account, and the escrow account is a PDA owned by the program, 
        // we can use the same seeds to sign for both the transfer and the close account CPI calls
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.escrow.creator.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump]
        ]];

        let cpi_ctx = CpiContext::new_with_signer(self.token_program.to_account_info(), cpi_accounts, &signer_seeds);

        transfer_checked(
            cpi_ctx, 
            self.escrow.receive, 
            self.mint_a.decimals
        )?;


        // closes vault account, rent is returned to creator by CPI 'close_account'
        let cpi_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.creator.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let cpi_context = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            cpi_accounts, 
            &signer_seeds
        );

        close_account(cpi_context)
    }
}