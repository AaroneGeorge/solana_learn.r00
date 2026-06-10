use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin, // Function to read plugin data from an NFT
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, PluginType, UpdateAuthority,
    },
    ID as MPL_CORE_ID,
};

use crate::{error::ErrorCode, state::Config};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        has_one = owner @ ErrorCode::InvalidOwner,
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ ErrorCode::InvalidUpdateAuthority,
        // checks update authority of NFT is the collection
    )]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(
        mut,
        has_one = update_authority @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<Stake>) -> Result<()> {
    // We start by fetching the existing attributes (if they exist)
    let attributes_fetched = fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    )
    .ok()
    .map(|(_, attrs, _)| attrs);

    //Prepare the Attributes list to add or update based on the existing attributes
    let mut attributes_list: Vec<Attribute> = Vec::new();

    // Loop to all attributes and save only the ones that are not the Staking attributes ("staked" and "staked_at")
    // If we find the "staked" attribute already present, we need to make sure the asset is not already staked
    if let Some(attributes) = &attributes_fetched {
        for attribute in &attributes.attribute_list {
            if attribute.key == "staked" {
                require!(attribute.value == "false", ErrorCode::AlreadyStaked);
            } else if attribute.key != "staked_at" {
                attributes_list.push(attribute.clone());
            }
        }
    }

    //Add the Staking attributes
    attributes_list.push(Attribute {
        key: "staked".to_string(),
        value: "true".to_string(),
    });

    attributes_list.push(Attribute {
        key: "staked_at".to_string(),
        value: Clock::get()?.unix_timestamp.to_string(),
    });

    // Now that we have the complete list of Attributes we either add the Plugin or Update the existing one
    // The Attributes Plugin is an Authority-Managed Plugin, so it needs to be signed by the update authority(PDA of the program)

    // signing seeds for the update authority
    let collection_key = ctx.accounts.collection.key();
    let signer_seeds = &[
        b"update_authority",
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];

    //If the Attributes Plugin does not exist, we need it
    if attributes_fetched.is_none() {
        AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.owner.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(Plugin::Attributes(Attributes {
                attribute_list: attributes_list,
            }))
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke_signed(&[signer_seeds])?;
    }
    //If the Attributes Plugin exists, we update it
    else {
        UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.owner.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(Plugin::Attributes(Attributes {
                attribute_list: attributes_list,
            }))
            .invoke_signed(&[signer_seeds])?;
    }

    // Freeze the asset with the FreezeDelegate Plugin
    // FreezeDelegate is an Owner-Managed Plugin, signed by the owner
    AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.owner.to_account_info())
        .authority(Some(&ctx.accounts.owner.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
        .init_authority(PluginAuthority::Owner)
        .invoke()?;

    Ok(())
}

// Stake Flow
//   1. Validate — Ensure caller is owner and NFT is not already staked
//   2. Add/Update Attributes Plugin with staked = true and staked_at = now
//   3. Add FreezeDelegate Plugin to freeze the NFT (prevent transfers while staked)
//   4. Records timestamps - logs when staking began, useful for rewards calculation on unstake
