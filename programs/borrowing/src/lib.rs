use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use borsh::{BorshDeserialize, BorshSerialize};
use decimal_wad::error::DecimalError;

mod token_operations;
use token_operations::{soltoken, stablecoin};

pub mod utils;
use utils::{bn::U256, pda};

mod borrowing_market;
mod handler_add_redemption_order;
mod handler_approve_staking_pool;
mod handler_approve_trove;
mod handler_borrow_stablecoin;
mod handler_clear_liquidation_gains;
mod handler_clear_redemption_order;
mod handler_deposit_and_borrow;
mod handler_deposit_collateral;
mod handler_fill_redemption_order;
mod handler_harvest_liquidation_gains;
mod handler_harvest_staking_reward;
mod handler_initialize_borrowing_market;
mod handler_initialize_stability_pool;
mod handler_initialize_staking_pool;
mod handler_repay_loan;
mod handler_serum_close_account;
mod handler_serum_init_account;
mod handler_serum_swap;
mod handler_stability_approve;
mod handler_stability_provide;
mod handler_stability_withdraw;
mod handler_stake_hbb;
mod handler_try_liquidate;
mod handler_unstake_hbb;
mod handler_update_global_config;
mod handler_withdraw_collateral;
pub mod redemption;
mod stability_pool;
mod staking_pool;

pub mod state;

use state::*;

use crate::state::CollateralToken;
pub use borrowing_market::borrowing_operations::apply_pending_rewards;

declare_id!("8v1DhJaewvhbhDmptNrkYig7YFcExsRKteR3cYjLw2iy");

#[program]
pub mod borrowing {

    use super::*;

    pub fn initialize_borrowing_market(ctx: Context<InitializeBorrowingMarket>) -> ProgramResult {
        // good to go
        handler_initialize_borrowing_market::process(ctx)
    }

    pub fn update_global_config(
        ctx: Context<UpdateGlobalConfig>,
        key: u16,
        value: u64,
    ) -> ProgramResult {
        handler_update_global_config::process(ctx, key, value)
    }

    pub fn approve_trove(ctx: Context<ApproveTrove>) -> ProgramResult {
        // good to go
        handler_approve_trove::process(ctx)
    }

    pub fn deposit_collateral(
        ctx: Context<DepositCollateral>,
        amount_in_lamports: u64,
        collateral: u8,
    ) -> ProgramResult {
        // good to go (maybe change wrappedsol)
        msg!("Depositing {} {}", amount_in_lamports, collateral);
        handler_deposit_collateral::process(
            ctx,
            amount_in_lamports,
            CollateralToken::from(collateral),
        )
    }

    pub fn borrow_stablecoin(ctx: Context<BorrowStable>, amount: u64) -> ProgramResult {
        // good to go
        handler_borrow_stablecoin::process(ctx, amount)
    }

    pub fn deposit_collateral_and_borrow_stablecoin(
        ctx: Context<DepositCollateralAndBorrowStable>,
        deposit_amount: u64,
        deposit_asset: u8,
        borrow_amount: u64,
    ) -> ProgramResult {
        handler_deposit_and_borrow::process(
            ctx,
            deposit_amount,
            CollateralToken::from(deposit_asset),
            borrow_amount,
        )
    }

    pub fn repay_loan(ctx: Context<RepayLoan>, amount: u64) -> ProgramResult {
        handler_repay_loan::process(ctx, amount)
    }

    pub fn withdraw_collateral(
        ctx: Context<WithdrawCollateral>,
        amount: u64,
        collateral: u8,
    ) -> ProgramResult {
        // good to go
        handler_withdraw_collateral::process(ctx, amount, CollateralToken::from(collateral))
    }

    pub fn stability_initialize(ctx: Context<InitializeStabilityPool>) -> ProgramResult {
        // good to go
        handler_initialize_stability_pool::process(ctx)
    }

    pub fn stability_approve(ctx: Context<ApproveProvideStability>) -> ProgramResult {
        // good to go
        handler_stability_approve::process(ctx)
    }

    pub fn stability_provide(ctx: Context<ProvideStability>, amount: u64) -> ProgramResult {
        // good to go
        handler_stability_provide::process(ctx, amount)
    }

    pub fn stability_withdraw(ctx: Context<WithdrawStability>, amount: u64) -> ProgramResult {
        // good to go
        handler_stability_withdraw::process(ctx, amount)
    }

    pub fn try_liquidate(ctx: Context<TryLiquidate>) -> ProgramResult {
        // good to go
        // might add seed generated addresses to remove the fixed size queue altogether
        handler_try_liquidate::process(ctx)
    }

    pub fn harvest_liquidation_gains(
        ctx: Context<HarvestLiquidationGains>,
        token: u8,
    ) -> ProgramResult {
        // good to go (might be subject to above change)
        handler_harvest_liquidation_gains::process(ctx, StabilityToken::from(token))
    }

    pub fn clear_liquidation_gains(
        ctx: Context<ClearLiquidationGains>,
        token: u8,
    ) -> ProgramResult {
        // good to go (might be subject to above change)
        handler_clear_liquidation_gains::process(ctx, CollateralToken::from(token))
    }

    pub fn add_redemption_order(
        ctx: Context<AddRedemptionOrder>,
        stablecoin_amount: u64,
    ) -> ProgramResult {
        // 95%
        // block redemptions when system is in Recovery mode && change the dynamic rate depeding on Recovery mode
        handler_add_redemption_order::process(ctx, stablecoin_amount)
    }

    pub fn fill_redemption_order(
        ctx: Context<FillRedemptionOrder>,
        order_id: u64,
    ) -> ProgramResult {
        // good to go
        handler_fill_redemption_order::process(ctx, order_id)
    }

    pub fn clear_redemption_order(
        ctx: Context<ClearRedemptionOrder>,
        order_id: u64,
    ) -> ProgramResult {
        // good to go
        handler_clear_redemption_order::process(ctx, order_id)
    }

    pub fn staking_initialize(
        ctx: Context<InitializeStakingPool>,
        treasury_fee_rate: u16,
    ) -> ProgramResult {
        // good to go
        handler_initialize_staking_pool::process(ctx, treasury_fee_rate)
    }

    pub fn staking_approve(ctx: Context<ApproveStakingPool>) -> ProgramResult {
        // good to go
        handler_approve_staking_pool::process(ctx)
    }

    pub fn staking_stake_hbb(ctx: Context<StakeHbbStakingPool>, amount: u64) -> ProgramResult {
        // good to go
        handler_stake_hbb::process(ctx, amount)
    }

    pub fn staking_harvest_reward(ctx: Context<HarvestRewardStakingPool>) -> ProgramResult {
        // good to go
        // might change something here - gain: u64 -> TokenMap
        handler_harvest_staking_reward::process(ctx)
    }

    pub fn unstake_hbb(ctx: Context<UnstakeHbbStakingPool>, amount: u64) -> ProgramResult {
        // good to go
        handler_unstake_hbb::process(ctx, amount)
    }

    pub fn serum_init_account(ctx: Context<SerumInitOpenOrders>) -> ProgramResult {
        handler_serum_init_account::process(ctx)
    }

    pub fn serum_swap_usdc(
        ctx: Context<SerumSwapToUsdc>,
        side: u8,
        max_pc_qty: u64,
        collateral: u8,
    ) -> ProgramResult {
        handler_serum_swap::process(ctx, side, max_pc_qty, CollateralToken::from(collateral))
    }

    pub fn serum_close_account(ctx: Context<SerumCloseOpenOrders>) -> ProgramResult {
        handler_serum_close_account::process(ctx)
    }

    pub fn airdrop_hbb(ctx: Context<AirdropHbb>, amount: u64) -> ProgramResult {
        // admin - initialMarketOwner
        let borrowing_market_state = &ctx.accounts.borrowing_market_state;
        token_operations::hbb::mint(
            amount,
            borrowing_market_state.hbb_mint_seed,
            borrowing_market_state.initial_market_owner,
            ctx.program_id,
            ctx.accounts.hbb_mint.clone(),
            ctx.accounts.user_hbb_ata.clone(),
            ctx.accounts.hbb_mint_authority.clone(),
            ctx.accounts.token_program.to_account_info(),
        )
    }

    pub fn airdrop_usdh(ctx: Context<AirdropUsdh>, amount: u64) -> ProgramResult {
        // will go away
        // ignore this for next 2 weeks - and will revert with a decision on it
        // cfg(not(local_validator))]
        // cfg(not(local_validator))]
        ctx.accounts.borrowing_market_state.stablecoin_borrowed += amount;
        stablecoin::mint(
            amount,
            ctx.accounts.borrowing_market_state.stablecoin_mint_seed,
            ctx.accounts.borrowing_market_state.initial_market_owner,
            ctx.program_id,
            ctx.accounts.stablecoin_mint.clone(),
            ctx.accounts.stablecoin_ata.clone(),
            ctx.accounts.stablecoin_mint_authority.clone(),
            ctx.accounts.token_program.to_account_info(),
        )
    }
}

#[derive(Accounts)]
pub struct InitializeBorrowingMarket<'info> {
    #[account(signer)]
    pub initial_market_owner: AccountInfo<'info>,

    #[account(init, payer = initial_market_owner)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(init, payer = initial_market_owner)]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(init, payer = initial_market_owner)]
    pub global_config: ProgramAccount<'info, GlobalConfig>,

    #[account(zero)]
    pub redemptions_queue: Loader<'info, RedemptionsQueue>,

    // Where all the borrowing fees are directed
    #[account(mut)]
    pub borrowing_fees_vault: AccountInfo<'info>,

    // Where all the repaid debt is directed and burned
    #[account(mut)]
    pub burning_vault: AccountInfo<'info>,

    // Vaults where you deploy
    #[account(mut)]
    pub collateral_vault_sol: AccountInfo<'info>,
    #[account(mut)]
    pub collateral_vault_srm: AccountInfo<'info>,
    #[account(mut)]
    pub collateral_vault_eth: AccountInfo<'info>,
    #[account(mut)]
    pub collateral_vault_btc: AccountInfo<'info>,
    #[account(mut)]
    pub collateral_vault_ray: AccountInfo<'info>,
    #[account(mut)]
    pub collateral_vault_ftt: AccountInfo<'info>,

    // Stablecoin account from which we mint/burn stablecoin
    #[account(mut)]
    pub stablecoin_mint: AccountInfo<'info>,

    // Hbb account from which we mint HBB
    #[account(mut)]
    pub hbb_mint: AccountInfo<'info>,

    pub srm_mint: AccountInfo<'info>,
    pub eth_mint: AccountInfo<'info>,
    pub btc_mint: AccountInfo<'info>,
    pub ray_mint: AccountInfo<'info>,
    pub ftt_mint: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateGlobalConfig<'info> {
    #[account(signer)]
    pub initial_market_owner: AccountInfo<'info>,

    #[account(mut, has_one = initial_market_owner)]
    pub global_config: ProgramAccount<'info, GlobalConfig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeStabilityPool<'info> {
    #[account(signer)]
    pub initial_market_owner: AccountInfo<'info>,

    #[account(has_one = initial_market_owner)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(init, payer = initial_market_owner)]
    pub stability_pool_state: ProgramAccount<'info, StabilityPoolState>,

    #[account(init, payer = initial_market_owner)]
    pub stability_vaults: ProgramAccount<'info, StabilityVaults>,

    #[account(zero)]
    pub epoch_to_scale_to_sum: Loader<'info, EpochToScaleToSumAccount>,

    #[account(zero)]
    pub liquidations_queue: Loader<'info, LiquidationsQueue>,

    // Vaults where liquidation gains are accumulated
    #[account(mut)]
    pub liquidation_rewards_vault_sol: AccountInfo<'info>,
    #[account(mut)]
    pub liquidation_rewards_vault_srm: AccountInfo<'info>,
    #[account(mut)]
    pub liquidation_rewards_vault_eth: AccountInfo<'info>,
    #[account(mut)]
    pub liquidation_rewards_vault_btc: AccountInfo<'info>,
    #[account(mut)]
    pub liquidation_rewards_vault_ray: AccountInfo<'info>,
    #[account(mut)]
    pub liquidation_rewards_vault_ftt: AccountInfo<'info>,

    #[account(mut)]
    pub stablecoin_stability_pool_vault: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct InitializeStakingPool<'info> {
    #[account(signer)]
    pub initial_market_owner: AccountInfo<'info>,

    #[account(has_one = initial_market_owner)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(init, payer = initial_market_owner)]
    pub staking_pool_state: ProgramAccount<'info, StakingPoolState>,

    #[account(mut)]
    pub staking_vault: AccountInfo<'info>,

    pub treasury_vault: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ApproveProvideStability<'info> {
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    #[account(init, payer = owner)]
    pub stability_provider_state: ProgramAccount<'info, StabilityProviderState>,

    #[account(mut)]
    pub stability_pool_state: ProgramAccount<'info, StabilityPoolState>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ProvideStability<'info> {
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        has_one = stability_pool_state,
    )]
    pub stability_provider_state: ProgramAccount<'info, StabilityProviderState>,

    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(mut,
        has_one = borrowing_market_state,
        has_one = epoch_to_scale_to_sum,
    )]
    pub stability_pool_state: ProgramAccount<'info, StabilityPoolState>,

    #[account(mut,
        has_one = stability_pool_state,
        has_one = stablecoin_stability_pool_vault,
    )]
    pub stability_vaults: ProgramAccount<'info, StabilityVaults>,

    #[account(mut)]
    pub epoch_to_scale_to_sum: Loader<'info, EpochToScaleToSumAccount>,

    #[account(mut)]
    pub stablecoin_stability_pool_vault: AccountInfo<'info>,

    // must be an owner ATA
    #[account(mut)]
    pub stablecoin_ata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct WithdrawStability<'info> {
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        has_one = stability_pool_state,
    )]
    pub stability_provider_state: ProgramAccount<'info, StabilityProviderState>,

    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(mut,
        has_one = borrowing_market_state,
        has_one = epoch_to_scale_to_sum,
    )]
    pub stability_pool_state: ProgramAccount<'info, StabilityPoolState>,

    #[account(mut,
        has_one = stability_pool_state,
        has_one = stablecoin_stability_pool_vault,
        has_one = stablecoin_stability_pool_vault_authority,
    )]
    pub stability_vaults: ProgramAccount<'info, StabilityVaults>,

    #[account(mut)]
    pub epoch_to_scale_to_sum: Loader<'info, EpochToScaleToSumAccount>,

    #[account(mut)]
    pub stablecoin_stability_pool_vault: AccountInfo<'info>,
    pub stablecoin_stability_pool_vault_authority: AccountInfo<'info>,

    // must be owner ATA
    #[account(mut)]
    pub stablecoin_ata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct ApproveStakingPool<'info> {
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    #[account(init, payer = owner)]
    pub user_staking_state: ProgramAccount<'info, UserStakingState>,
    #[account(mut)]
    pub staking_pool_state: ProgramAccount<'info, StakingPoolState>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct StakeHbbStakingPool<'info> {
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        has_one = staking_pool_state,
    )]
    pub user_staking_state: ProgramAccount<'info, UserStakingState>,

    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(mut,
        has_one = borrowing_market_state,
        has_one = staking_vault,
    )]
    pub staking_pool_state: ProgramAccount<'info, StakingPoolState>,

    #[account(mut)]
    pub staking_vault: AccountInfo<'info>,

    // must be owner ATA
    #[account(mut)]
    pub user_hbb_staking_ata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UnstakeHbbStakingPool<'info> {
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        has_one = staking_pool_state,
    )]
    pub user_staking_state: ProgramAccount<'info, UserStakingState>,

    #[account(mut)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(
        has_one = borrowing_market_state,
        has_one = borrowing_fees_vault,
        has_one = borrowing_fees_vault_authority,
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut,
        has_one = borrowing_market_state,
        has_one = staking_vault,
        has_one = staking_vault_authority,
    )]
    pub staking_pool_state: ProgramAccount<'info, StakingPoolState>,

    // Must be the user's ATA
    #[account(mut)]
    pub user_hbb_staking_ata: AccountInfo<'info>,
    // Must be the user's ATA
    #[account(mut)]
    pub user_stablecoin_rewards_ata: AccountInfo<'info>,

    #[account(mut)]
    pub staking_vault: AccountInfo<'info>,
    pub staking_vault_authority: AccountInfo<'info>,

    #[account(mut)]
    pub borrowing_fees_vault: AccountInfo<'info>,
    pub borrowing_fees_vault_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct HarvestRewardStakingPool<'info> {
    #[account(signer, mut)]
    pub owner: AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        has_one = staking_pool_state,
    )]
    pub user_staking_state: ProgramAccount<'info, UserStakingState>,

    #[account(mut)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(
        has_one = borrowing_market_state,
        has_one = borrowing_fees_vault,
        has_one = borrowing_fees_vault_authority,
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut,
        has_one = borrowing_market_state,
    )]
    pub staking_pool_state: ProgramAccount<'info, StakingPoolState>,

    // Must be the user's ATA
    #[account(mut)]
    pub user_stablecoin_rewards_ata: AccountInfo<'info>,

    #[account(mut)]
    pub borrowing_fees_vault: AccountInfo<'info>,
    pub borrowing_fees_vault_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ApproveTrove<'info> {
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    #[account(init, payer = owner)]
    pub user_metadata: ProgramAccount<'info, UserMetadata>,

    #[account(mut)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    pub stablecoin_ata: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    #[account(mut)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(has_one = borrowing_market_state)]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut,
        has_one = owner,
        has_one = borrowing_market_state
    )]
    pub user_metadata: ProgramAccount<'info, UserMetadata>,

    // Account where collateral is deposited from, either the owner account or an ATA
    #[account(mut)]
    pub collateral_from: AccountInfo<'info>,

    // Vault where collateral is deposited to
    #[account(mut)]
    pub collateral_to: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BorrowStable<'info> {
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    // Global state
    #[account(mut,
        has_one = stablecoin_mint,
        has_one = stablecoin_mint_authority
    )]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(
        has_one = borrowing_market_state,
        has_one = borrowing_fees_vault
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    //Staking pool state to update rewards data
    #[account(mut,
        has_one = borrowing_market_state,
        has_one = treasury_vault
    )]
    pub staking_pool_state: ProgramAccount<'info, StakingPoolState>,

    #[account(mut,
        has_one = owner,
        has_one = borrowing_market_state
    )]
    pub user_metadata: ProgramAccount<'info, UserMetadata>,

    // Stablecoin account from which we mint/burn stablecoin
    #[account(mut)]
    pub stablecoin_mint: AccountInfo<'info>,

    // Source of stablecoin mint PDA authority (authority to which minting rights have been transferred)
    pub stablecoin_mint_authority: AccountInfo<'info>,

    // Where stablecoin will be minted (borrowed)
    #[account(mut,
        constraint = stablecoin_borrowing_associated_account.key == &user_metadata.stablecoin_ata
    )]
    pub stablecoin_borrowing_associated_account: AccountInfo<'info>,

    // Where the borrowing 0.5% fees go
    #[account(mut)]
    pub borrowing_fees_vault: AccountInfo<'info>,

    // Where the treasury fees go
    #[account(mut)]
    pub treasury_vault: AccountInfo<'info>,

    // Oracle accounts
    pub pyth_sol_price_info: AccountInfo<'info>,
    pub pyth_eth_price_info: AccountInfo<'info>,
    pub pyth_btc_price_info: AccountInfo<'info>,
    pub pyth_srm_price_info: AccountInfo<'info>,
    pub pyth_ray_price_info: AccountInfo<'info>,
    pub pyth_ftt_price_info: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct DepositCollateralAndBorrowStable<'info> {
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    // Global state
    #[account(mut,
        has_one = stablecoin_mint,
        has_one = stablecoin_mint_authority
    )]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(
        has_one = borrowing_market_state,
        has_one = borrowing_fees_vault
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    //Staking pool state to update rewards data
    #[account(mut,
        has_one = borrowing_market_state,
        has_one = treasury_vault
    )]
    pub staking_pool_state: ProgramAccount<'info, StakingPoolState>,

    #[account(mut,
        has_one = owner,
        has_one = borrowing_market_state
    )]
    pub user_metadata: ProgramAccount<'info, UserMetadata>,

    // Stablecoin account from which we mint/burn stablecoin
    #[account(mut)]
    pub stablecoin_mint: AccountInfo<'info>,

    // Source of stablecoin mint PDA authority (authority to which minting rights have been transferred)
    pub stablecoin_mint_authority: AccountInfo<'info>,

    // Account where collateral is deposited from, either the owner account or an ATA
    #[account(mut)]
    pub collateral_from: AccountInfo<'info>,

    // Vault where collateral is deposited to
    #[account(mut)]
    pub collateral_to: AccountInfo<'info>,

    // Where stablecoin will be minted (borrowed)
    #[account(mut,
        constraint = stablecoin_borrowing_associated_account.key == &user_metadata.stablecoin_ata
    )]
    pub stablecoin_borrowing_associated_account: AccountInfo<'info>,

    // Where the borrowing 0.5% fees go
    #[account(mut)]
    pub borrowing_fees_vault: AccountInfo<'info>,

    // Where the treasury fees go
    #[account(mut)]
    pub treasury_vault: AccountInfo<'info>,

    // Oracle accounts
    pub pyth_sol_price_info: AccountInfo<'info>,
    pub pyth_eth_price_info: AccountInfo<'info>,
    pub pyth_btc_price_info: AccountInfo<'info>,
    pub pyth_srm_price_info: AccountInfo<'info>,
    pub pyth_ray_price_info: AccountInfo<'info>,
    pub pyth_ftt_price_info: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct RepayLoan<'info> {
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    // Global state
    #[account(mut,
        has_one = stablecoin_mint,
        has_one = stablecoin_mint_authority,
    )]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,
    #[account(mut,
        has_one = borrowing_market_state,
        has_one = burning_vault,
        has_one = burning_vault_authority,
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut,
        has_one = owner,
        has_one = borrowing_market_state,
    )]
    pub user_metadata: ProgramAccount<'info, UserMetadata>,

    // Stablecoin account from which we mint/burn stablecoin
    #[account(mut)]
    pub stablecoin_mint: AccountInfo<'info>,
    // Source of stablecoin mint PDA authority (authority to which minting rights have been transferred)
    pub stablecoin_mint_authority: AccountInfo<'info>,

    // Where stablecoin will be repaid from
    #[account(mut,
        constraint = stablecoin_borrowing_associated_account.key == &user_metadata.stablecoin_ata
    )]
    pub stablecoin_borrowing_associated_account: AccountInfo<'info>,

    // Where the debt will be repaid into (and burned)
    #[account(mut)]
    pub burning_vault: AccountInfo<'info>,
    pub burning_vault_authority: AccountInfo<'info>,

    // Source of stablecoin mint
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    #[account(mut)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(has_one = borrowing_market_state)]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut,
        has_one = owner,
        has_one = borrowing_market_state
    )]
    pub user_metadata: ProgramAccount<'info, UserMetadata>,

    // Vault where collateral is withdrawn from
    #[account(mut)]
    pub collateral_from: AccountInfo<'info>,
    pub collateral_from_authority: AccountInfo<'info>,

    // Where collateral is withdrawn to
    // Must be the user's mint ATA or native account
    #[account(mut)]
    pub collateral_to: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,

    // Oracle accounts
    pub pyth_sol_price_info: AccountInfo<'info>,
    pub pyth_eth_price_info: AccountInfo<'info>,
    pub pyth_btc_price_info: AccountInfo<'info>,
    pub pyth_srm_price_info: AccountInfo<'info>,
    pub pyth_ray_price_info: AccountInfo<'info>,
    pub pyth_ftt_price_info: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct AddRedemptionOrder<'info> {
    #[account(mut, signer)]
    pub redeemer: AccountInfo<'info>,

    #[account(mut,
        constraint = redeemer.key == &redeemer_metadata.owner,
        has_one = borrowing_market_state,
    )]
    pub redeemer_metadata: ProgramAccount<'info, UserMetadata>,

    // Must be redeemer ATA
    #[account(mut,
        constraint = redeemer_stablecoin_associated_account.key == &redeemer_metadata.stablecoin_ata,
    )]
    pub redeemer_stablecoin_associated_account: AccountInfo<'info>,

    #[account(mut,
        has_one = redemptions_queue,
    )]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(
        has_one = borrowing_market_state,
        has_one = burning_vault,
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut)]
    pub redemptions_queue: Loader<'info, RedemptionsQueue>,

    #[account(mut)]
    pub burning_vault: AccountInfo<'info>,

    pub pyth_sol_price_info: AccountInfo<'info>,
    pub pyth_eth_price_info: AccountInfo<'info>,
    pub pyth_btc_price_info: AccountInfo<'info>,
    pub pyth_srm_price_info: AccountInfo<'info>,
    pub pyth_ray_price_info: AccountInfo<'info>,
    pub pyth_ftt_price_info: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct FillRedemptionOrder<'info> {
    #[account(signer)]
    pub filler: AccountInfo<'info>,

    #[account(mut,
        constraint = filler.key == &filler_metadata.owner,
        has_one = borrowing_market_state,
    )]
    pub filler_metadata: ProgramAccount<'info, UserMetadata>,

    #[account(mut,
        has_one = redemptions_queue,
    )]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,
    #[account(mut)]
    pub redemptions_queue: Loader<'info, RedemptionsQueue>,

    pub clock: Sysvar<'info, Clock>,
    // remaining accounts (user metadata, candidate user)
}

#[derive(Accounts)]
pub struct ClearRedemptionOrder<'info> {
    #[account(signer)]
    pub clearer: AccountInfo<'info>,

    #[account(mut,
        constraint = clearer.key == &clearer_metadata.owner,
        has_one = borrowing_market_state,
    )]
    pub clearer_metadata: ProgramAccount<'info, UserMetadata>,

    #[account(mut,
        has_one = borrowing_market_state,
    )]
    pub redeemer_metadata: ProgramAccount<'info, UserMetadata>,

    #[account(mut,
        has_one = redemptions_queue,
        has_one = stablecoin_mint,
    )]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(
        has_one = borrowing_market_state,
        has_one = burning_vault,
        has_one = burning_vault_authority,
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut)]
    pub redemptions_queue: Loader<'info, RedemptionsQueue>,

    #[account(mut)]
    pub burning_vault: AccountInfo<'info>,
    pub burning_vault_authority: AccountInfo<'info>,
    #[account(mut)]
    pub stablecoin_mint: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    // remaining accounts (user metadata, borrowers and fillers)
}

#[derive(Accounts)]
pub struct TryLiquidate<'info> {
    #[account(signer, mut)]
    pub liquidator: AccountInfo<'info>,

    #[account(mut,
        has_one = stablecoin_mint,
        has_one = stablecoin_mint_authority,
    )]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,
    #[account(mut,
        has_one = borrowing_market_state,
        has_one = epoch_to_scale_to_sum,
        has_one = liquidations_queue,
    )]
    pub stability_pool_state: ProgramAccount<'info, StabilityPoolState>,
    #[account(mut,
        has_one = borrowing_market_state,
    )]
    pub user_metadata: ProgramAccount<'info, UserMetadata>,

    #[account(mut)]
    pub epoch_to_scale_to_sum: Loader<'info, EpochToScaleToSumAccount>,
    #[account(mut,
        has_one = stability_pool_state,
        has_one = stablecoin_stability_pool_vault,
        has_one = stablecoin_stability_pool_vault_authority,
    )]
    pub stability_vaults: ProgramAccount<'info, StabilityVaults>,
    #[account(
        has_one = borrowing_market_state,
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,
    #[account(mut)]
    pub liquidations_queue: Loader<'info, LiquidationsQueue>,

    // Stablecoin account from which we mint/burn stablecoin
    #[account(mut)]
    pub stablecoin_mint: AccountInfo<'info>,
    pub stablecoin_mint_authority: AccountInfo<'info>,

    #[account(mut)]
    pub stablecoin_stability_pool_vault: AccountInfo<'info>,
    pub stablecoin_stability_pool_vault_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,

    // todo - security - verify oracle accs
    pub pyth_sol_price_info: AccountInfo<'info>,
    pub pyth_eth_price_info: AccountInfo<'info>,
    pub pyth_btc_price_info: AccountInfo<'info>,
    pub pyth_srm_price_info: AccountInfo<'info>,
    pub pyth_ray_price_info: AccountInfo<'info>,
    pub pyth_ftt_price_info: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct HarvestLiquidationGains<'info> {
    #[account(signer, mut)]
    pub owner: AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        has_one = stability_pool_state,
    )]
    pub stability_provider_state: ProgramAccount<'info, StabilityProviderState>,

    #[account(
        has_one = hbb_mint,
        has_one = hbb_mint_authority,
    )]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(
        has_one = borrowing_market_state,
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut,
        has_one = borrowing_market_state,
        has_one = epoch_to_scale_to_sum,
        has_one = liquidations_queue,
    )]
    pub stability_pool_state: ProgramAccount<'info, StabilityPoolState>,

    #[account(
        has_one = stability_pool_state,
        has_one = liquidation_rewards_vault_authority,
    )]
    pub stability_vaults: ProgramAccount<'info, StabilityVaults>,

    #[account(mut)]
    pub liquidations_queue: Loader<'info, LiquidationsQueue>,
    #[account(mut)]
    pub epoch_to_scale_to_sum: Loader<'info, EpochToScaleToSumAccount>,

    // Where rewards are withdrawn from
    #[account(mut)]
    pub liquidation_rewards_vault: AccountInfo<'info>,
    pub liquidation_rewards_vault_authority: AccountInfo<'info>,
    // Where rewards are sent to
    // Must be the stability provider's mint ATA or native account
    #[account(mut)]
    pub liquidation_rewards_to: AccountInfo<'info>,

    #[account(mut)]
    pub hbb_mint: AccountInfo<'info>,
    pub hbb_mint_authority: AccountInfo<'info>,
    // Where HBB rewards are sent to
    // Must be the stability provider's HBB ATA
    #[account(mut)]
    pub hbb_ata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct ClearLiquidationGains<'info> {
    // It's not necessary for the liquidator to call this
    // Can be any bot
    #[account(mut, signer)]
    pub clearing_agent: AccountInfo<'info>,

    // The ATA where the clearing gets their gains for
    // handling the liquidation event or clearing the queue
    // clearing agent is most of the times the liquidator
    // Must be the clearing agent's mint ATA or native account
    #[account(mut)]
    pub clearing_agent_ata: AccountInfo<'info>,

    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(
        has_one = borrowing_market_state,
        has_one = collateral_vaults_authority,
    )]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut,
        has_one = borrowing_market_state,
        has_one = liquidations_queue,
    )]
    pub stability_pool_state: ProgramAccount<'info, StabilityPoolState>,

    #[account(
        has_one = stability_pool_state,
    )]
    pub stability_vaults: ProgramAccount<'info, StabilityVaults>,

    // Liquidations queue where we have recorded the liquidation event
    #[account(mut)]
    pub liquidations_queue: Loader<'info, LiquidationsQueue>,

    // Collateral vault from which the token is moved
    #[account(mut)]
    pub collateral_vault: AccountInfo<'info>,
    pub collateral_vaults_authority: AccountInfo<'info>,
    // Rewards vault where the gains are going for the stability pool for one token
    #[account(mut)]
    pub liquidation_rewards_vault: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

// Maybe remove this
#[derive(Accounts)]
pub struct AirdropHbb<'info> {
    #[account(signer, mut)]
    pub initial_market_owner: AccountInfo<'info>,

    #[account(has_one = initial_market_owner)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(mut)]
    pub user_hbb_ata: AccountInfo<'info>,

    #[account(mut)]
    pub hbb_mint: AccountInfo<'info>,
    pub hbb_mint_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AirdropUsdh<'info> {
    #[account(signer)]
    pub initial_market_owner: AccountInfo<'info>,

    #[account(has_one = initial_market_owner)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,

    #[account(mut)]
    pub stablecoin_ata: AccountInfo<'info>,

    #[account(mut)]
    pub stablecoin_mint: AccountInfo<'info>,
    pub stablecoin_mint_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts, Clone)]
pub struct SerumInitOpenOrders<'info> {
    pub dex_program: AccountInfo<'info>,
    #[account(mut)]
    pub open_orders: AccountInfo<'info>,
    #[account(signer)]
    pub order_payer_authority: AccountInfo<'info>,
    pub market: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts, Clone)]
pub struct SerumCloseOpenOrders<'info> {
    #[account(mut)]
    open_orders: AccountInfo<'info>,
    #[account(signer)]
    authority: AccountInfo<'info>,
    #[account(mut)]
    destination: AccountInfo<'info>,
    market: AccountInfo<'info>,
    dex_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SerumSwapToUsdc<'info> {
    /// The DEX program
    pub dex_program: AccountInfo<'info>,

    // The market pair address (A-Token/USDC)
    #[account(mut)]
    market: AccountInfo<'info>,

    // The account that stores the open orders queue
    #[account(mut)]
    open_orders: AccountInfo<'info>,

    // The account that initialized the open orders
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    // The queue with the events that are being processeds
    #[account(mut)]
    request_queue: AccountInfo<'info>,

    // The queue with the consumed events on the market
    #[account(mut)]
    event_queue: AccountInfo<'info>,

    // The address of bids on the orderbook
    #[account(mut)]
    bids: AccountInfo<'info>,

    // The address of asks on the orderbook
    #[account(mut)]
    asks: AccountInfo<'info>,

    /// The DEX vault for the "base" currency (WSOL/BTC/ETH, etc)
    #[account(mut)]
    coin_vault: AccountInfo<'info>,

    /// The DEX vault for the "quote" currency (USDC)
    #[account(mut)]
    pc_vault: AccountInfo<'info>,

    // An intermed account that will pay for the new order tx (we fund this account through the coll vault)
    #[account(mut)]
    dex_swap_account: AccountInfo<'info>,

    #[account(mut)]
    pub borrowing_market_state: ProgramAccount<'info, BorrowingMarketState>,
    #[account(has_one = borrowing_market_state)]
    pub borrowing_vaults: ProgramAccount<'info, BorrowingVaults>,

    #[account(mut,
        has_one = owner,
        has_one = borrowing_market_state
    )]
    pub user_metadata: ProgramAccount<'info, UserMetadata>,

    #[account(mut)]
    pub pc_wallet: AccountInfo<'info>,
    /// DEX owner
    vault_signer: AccountInfo<'info>,

    /// The ATA I expect to receive the base tokens back (WSOL) if the orders aren't executed (in this case, it's going to be the coll vault, that also executes the transfer)
    #[account(mut)]
    collateral_vault: AccountInfo<'info>,

    pub collateral_from_authority: AccountInfo<'info>,

    // Pass usdc_mint for security (verify that the user that receives USDC is the user_metadata owner)
    pub usdc_mint: AccountInfo<'info>,

    // Oracle accounts
    pub pyth_sol_price_info: AccountInfo<'info>,
    pub pyth_eth_price_info: AccountInfo<'info>,
    pub pyth_btc_price_info: AccountInfo<'info>,
    pub pyth_srm_price_info: AccountInfo<'info>,
    pub pyth_ray_price_info: AccountInfo<'info>,
    pub pyth_ftt_price_info: AccountInfo<'info>,

    pub token_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
}

#[error]
#[derive(PartialEq, Eq)]
pub enum BorrowError {
    #[msg("Insufficient collateral to cover debt")]
    NotEnoughCollateral,

    #[msg("Collateral not yet enabled")]
    CollateralNotEnabled,

    #[msg("Cannot deposit zero collateral amount")]
    CannotDepositZeroAmount,

    #[msg("Cannot withdraw zero collateral amount")]
    CannotWithdrawZeroAmount,

    #[msg("No outstanding debt")]
    NothingToRepay,

    #[msg("Could not generate seed")]
    CannotGenerateSeed,

    #[msg("Need to claim all rewards first")]
    NeedToClaimAllRewardsFirst,

    #[msg("Need to harvest all rewards first")]
    NeedToHarvestAllRewardsFirst,

    #[msg("Cannot stake or unstake 0 amount")]
    StakingZero,

    #[msg("Nothing to unstake")]
    NothingToUnstake,

    #[msg("Unstaking too much")]
    UnstakingTooMuch,

    #[msg("No reward to withdraw")]
    NoRewardToWithdraw,

    #[msg("Cannot provide zero stability")]
    CannotProvideZeroStability,

    #[msg("Stability Pool is empty")]
    StabilityPoolIsEmpty,

    #[msg("Stability pool cannot offset this much debt")]
    NotEnoughStabilityInTheStabilityPool,

    #[msg("Mismatching next PDA reward address")]
    MismatchedNextPdaRewardAddress,

    #[msg("Mismatching next PDA reward seed")]
    MismatchedNextPdaRewardSeed,

    #[msg("Wrong next reward pda index")]
    MismatchedNextPdaIndex,

    #[msg("Next reward not ready yet")]
    NextRewardNotReadyYet,

    #[msg("Nothing staked, cannot collect any rewards")]
    NothingStaked,

    #[msg("Reward candidate mismatch from user's next pending reward")]
    NextRewardMismatchForUser,

    #[msg("User is well collateralized, cannot liquidate")]
    UserWellCollateralized,

    #[msg("Cannot liquidate the last user")]
    LastUser,

    #[msg("Integer overflow")]
    IntegerOverflow,

    #[msg("Integer overflow")]
    ConversionFailure,

    #[msg("Cannot harvest until liquidation gains are cleared")]
    CannotHarvestUntilLiquidationGainsCleared,

    #[msg("Redemptions queue is full, cannot add one more order")]
    RedemptionsQueueIsFull,

    #[msg("Redemptions queue is empty, nothing to process")]
    RedemptionsQueueIsEmpty,

    #[msg("Redemptions amount too small")]
    RedemptionsAmountTooSmall,

    #[msg("Redemptions amount too much")]
    CannotRedeemMoreThanMinted,

    #[msg("The program needs to finish processing the first outstanding order before moving on to others")]
    NeedToProcessFirstOrderBeforeOthers,

    #[msg("The bot submitted the clearing users in the wrong order")]
    RedemptionClearingOrderIsIncorrect,

    #[msg("Current redemption order is in clearing mode, cannot fill it until it's fully cleared")]
    CannotFillRedemptionOrderWhileInClearingMode,

    #[msg("Current redemption order is in filling mode, cannot clear it until it's filled")]
    CannotClearRedemptionOrderWhileInFillingMode,

    #[msg("Redemption order is inactive")]
    InvalidRedemptionOrder,

    #[msg("Redemption order is empty of candidates")]
    OrderDoesNotHaveCandidates,

    #[msg("Redemption user is not among the candidates")]
    WrongRedemptionUser,

    #[msg("Redemption user is not among the candidates")]
    RedemptionFillerNotFound,

    #[msg("Redeemer does not match with the order being redeemed")]
    InvalidRedeemer,

    #[msg("Duplicate account in fill order")]
    DuplicateAccountInFillOrder,

    #[msg("Redemption user is not among the candidates")]
    RedemptionUserNotFound,

    #[msg("Mathematical operation with overflow")]
    MathOverflow,

    #[msg("Price is not valid")]
    PriceNotValid,

    #[msg("Liquidation queue is full")]
    LiquidationsQueueFull,

    #[msg("Liquidation queue is full")]
    CannotDeserializeSumMap,

    #[msg("Cannot borrow in Recovery mode")]
    CannotBorrowInRecoveryMode,

    #[msg("Cannot withdraw in Recovery mode")]
    CannotWithdrawInRecoveryMode,

    #[msg("Operation brings system to recovery mode")]
    OperationBringsSystemToRecoveryMode,

    #[msg("Cannot borrow zero amount")]
    CannotBorrowZeroAmount,

    #[msg("Cannot repay zero amount")]
    CannotRepayZeroAmount,

    #[msg("Cannot redeem during bootstrap period")]
    CannotRedeemDuringBootstrapPeriod,

    #[msg("Cannot borrow less than minimum")]
    CannotBorrowLessThanMinimum,

    #[msg("Debt is lower than minimum")]
    TooLowDebt,

    #[msg("Cannot redeem while being undercollateralized")]
    CannotRedeemWhenUndercollateralized,

    #[msg("Zero argument not allowed")]
    ZeroAmountInvalid,

    #[msg("Operation lowers system TCR")]
    OperationLowersSystemTCRInRecoveryMode,

    #[msg("Serum DEX variables inputted wrongly")]
    InvalidDexInputs,

    #[msg("Serum DEX transaction didn't swap")]
    ZeroSwap,

    #[msg("Key is not present in global config")]
    GlobalConfigKeyError,
}

impl From<DecimalError> for BorrowError {
    fn from(err: DecimalError) -> BorrowError {
        match err {
            DecimalError::MathOverflow => BorrowError::MathOverflow,
        }
    }
}
