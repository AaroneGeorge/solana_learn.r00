use anchor_lang::prelude::*;
use crate::state::Escrow;
use anchor_spl::{
    token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked, CloseAccount, close_account}
};


#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mint::token_program = token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,


    // Token A from the vault is refunded to this account (ATA) 
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = creator,
        associated_token::token_program = token_program,
    )]
    pub creator_ata_a: InterfaceAccount<'info, TokenAccount>,

    // closes the escrow account created by the creator.
    // has_one = creator ensures that the escrow account being closed is the one created by the creator and not any other escrow account
    #[account(
        mut,
        close = creator,
        has_one = mint_a,
        has_one = creator,
        seeds = [b"escrow", creator.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,

    // All token A will be withdrawn from this vault.
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>
}

impl<'info> Refund<'info> {
    pub fn refund_and_close_vault(&self) -> Result<()> {

        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.escrow.creator.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump]
        ]];

        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.creator_ata_a.to_account_info(),
            authority: self.escrow.to_account_info()
        };

        let cpi_context = CpiContext::new_with_signer(self.token_program.to_account_info(), cpi_accounts, &signer_seeds);

        transfer_checked(
            cpi_context, 
            self.vault.amount, 
            // Why vault.amount? instead of escrow.receieve?
            // escrow.recieve is used to transfer the agreed-upon amount
            // we use vault.amount to transfer whatever is in the vault, transfer full back to creator since we are closing the vault
            self.mint_a.decimals
        )?;

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