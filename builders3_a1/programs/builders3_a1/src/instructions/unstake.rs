use crate::error::ErrorCode;
use crate::state::Config;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        mint_to_checked, //Mint reward tokens to the user
        Mint,
        MintToChecked,
        TokenAccount,
        TokenInterface,
    },
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{Attribute, Attributes, FreezeDelegate, Plugin, PluginType, UpdateAuthority},
    ID as MPL_CORE_ID,
};

const SECONDS_PER_DAY: i64 = 86400;

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        has_one = owner @ ErrorCode::InvalidOwner,
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(
        mut,
        has_one = update_authority @ ErrorCode::InvalidUpdateAuthority
    )]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"rewards_mint", config.key().as_ref()],
        bump = config.rewards_bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner
    )]
    pub user_rewards_ata: InterfaceAccount<'info, TokenAccount>, // User's associated token account for rewards

    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Unstake>) -> Result<()> {
    // We start by fetching the existing attributes
    let attributes_fetched = fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    )
    .ok()
    .map(|(_, attrs, _)| attrs);

    // If attributes don't exist, the NFT was never staked, so unstaking is invalid
    require!(attributes_fetched.is_some(), ErrorCode::AssetNotStaked);

    let attributes = attributes_fetched.unwrap();

    //Prepare the Attributes list to update based on the existing attributes
    let mut attributes_list = Vec::with_capacity(attributes.attribute_list.len());

    //Additional auxiliary variables
    let current_timestamp = Clock::get()?.unix_timestamp;
    let mut staked_timestamp: i64 = 0;
    let mut staked_time: i64 = 0;

    // Loop through all attributes to:
    // a. Verify staked == "true" (else it's not staked)
    // b. Extract the staked_at timestamp
    // c. Keep all other attributes (to preserve use data)
    for attribute in &attributes.attribute_list {
        if attribute.key == "staked" {
            require!(attribute.value == "true", ErrorCode::AssetNotStaked);
        } else if attribute.key == "staked_at" {
            staked_timestamp = staked_timestamp
                .checked_add(
                    attribute
                        .value
                        .parse::<i64>()
                        .map_err(|_| ErrorCode::InvalidTimestamp)?,
                )
                .ok_or(ErrorCode::InvalidTimestamp)?;
            // calculate the time (in seconds) since the asset was staked
            staked_time = current_timestamp
                .checked_sub(staked_timestamp)
                .ok_or(ErrorCode::InvalidTimestamp)?;
            // Staked time in days
            staked_time = staked_time
                .checked_div(SECONDS_PER_DAY)
                .ok_or(ErrorCode::InvalidTimestamp)?;
            require!(
                staked_time >= ctx.accounts.config.freeze_period as i64,
                ErrorCode::FreezePeriodNotElapsed
            );
        } else {
            attributes_list.push(attribute.clone());
        }
    }

    // Prepare signing seeds for the update authority
    let collection_key = ctx.accounts.collection.key();
    let signer_seeds = &[
        b"update_authority",
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ];

    //Now we update the asset Attributes Plugin 

    //Add the Staking attributes first

    attributes_list.push(Attribute {
        key: "staked".to_string(),
        value: "false".to_string(),
    });

    attributes_list.push(Attribute {
        key: "staked_at".to_string(),
        value: "0".to_string(),
    });

    // The FreezeDelegate plugin prevents any modifications while frozen. We must unfreeze before updating attributes
    // FreezeDelegate is owner managed, so authority is the owner
    UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.owner.to_account_info())
        .authority(Some(&ctx.accounts.owner.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
        .invoke()?;

    // Now update the Attributes Plugin (asset is thawed, update_authority can sign)
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

    // Finally, we want to mint rewards to the user

    // Calculate the amount
    let amount = (staked_time as u64)
        .checked_mul(ctx.accounts.config.rewards_bps as u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_mul(10u64.pow(ctx.accounts.rewards_mint.decimals as u32))
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_div(10000u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?;

    // Prepare signer seeds for config PDA
    let config_seeds = &[
        b"config",
        collection_key.as_ref(),
        &[ctx.accounts.config.bump],
    ];

    let config_signer_seeds: &[&[&[u8]]; 1] = &[&config_seeds[..]];

    mint_to_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintToChecked {
                mint: ctx.accounts.rewards_mint.to_account_info(),
                to: ctx.accounts.user_rewards_ata.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            config_signer_seeds,
        ),
        amount,
        ctx.accounts.rewards_mint.decimals,
    )?;

    Ok(())
}

// Unstake Flow
//   1. Validate — Ensure NFT is actually staked
//   2. Calculate — Days staked and reward amount
//   3. Check — Verify freeze period has elapsed (can't unstake too early)
//   4. Unfreeze — Owner unfreezes the FreezeDelegate plugin
//   5. Reset — Set staked/staked_at back to false/0
//   6. Reward — Mint tokens to the user based on staking duration
//   7. Done — NFT is now unstaked and user has rewards!