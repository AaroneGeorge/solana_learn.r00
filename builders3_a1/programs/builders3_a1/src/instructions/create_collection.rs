use anchor_lang::prelude::*;
use mpl_core::{instructions::CreateCollectionV2CpiBuilder, ID as MPL_CORE_ID};

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub collection: Signer<'info>,

    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    /// CHECK: PDA derived from collection address, used to sign CPI to Metaplex for collection creation
    pub update_authority: UncheckedAccount<'info>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: Verified by address constraint against MPL_CORE_ID
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CreateCollection>, name: String, uri: String) -> Result<()> {
    // Stores the collection's public key (needed to derive the bump later)
    let collection_key = ctx.accounts.collection.key();

    let signer_seeds = &[
        b"update_authority",
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];

    CreateCollectionV2CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .collection(&ctx.accounts.collection.to_account_info())
        .payer(&ctx.accounts.payer.to_account_info())
        .update_authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .name(name)
        .uri(uri)
        .invoke_signed(&[signer_seeds])?;

    Ok(())
}

//   .collection() = the NFT collection being created
//   .payer() = who pays for the creation
//   .update_authority() = who can update this collection later (the PDA we created)
//   .name() / .uri() = metadata for the collection
