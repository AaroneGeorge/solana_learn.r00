pub use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account(discriminator = 1)] //1-255
pub struct Escrow{
    pub seed: u64,
    pub creator: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub receive: u64, // receieve amount
    pub bump: u8,
}