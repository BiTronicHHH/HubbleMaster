use anchor_lang::prelude::msg;
use std::{cell::RefMut, fmt::Debug};

use crate::{
    borrowing_market::borrowing_operations::utils::assert_not_zero,
    stability_pool::{liquidations_queue, stability_pool_operations},
    staking_pool::staking_pool_operations,
    state::{
        epoch_to_scale_to_sum::EpochToScaleToSum, LiquidationEvent, LiquidationsQueue, UserStatus,
    },
    utils::{
        consts::{BORROW_MIN, STABLECOIN_FACTOR},
        coretypes::CheckedAssign,
    },
    BorrowError, BorrowingMarketState, CollateralAmounts, CollateralToken, StabilityPoolState,
    StakingPoolState, TokenPrices, UserMetadata,
};
use anchor_lang::prelude::Pubkey;
use num::FromPrimitive;

#[derive(Debug)]
pub struct LiquidationBreakdownAmounts {
    pub usd_debt_to_redistribute: u64,
    pub usd_debt_to_stability_pool: u64,
    pub coll_to_redistribute: CollateralAmounts,
    pub coll_to_stability_pool: CollateralAmounts,
    pub coll_to_liquidator: CollateralAmounts,
    pub coll_to_clearer: CollateralAmounts,
}

pub struct UserBalances {
    pub user_current_debt: u64,
    pub user_pending_debt: u64,
    pub user_current_collateral: CollateralAmounts,
    pub user_pending_collateral: CollateralAmounts,
}

pub use self::redistribution::apply_pending_rewards;

use super::{
    borrowing_rate::{self, BorrowSplit, FeeEvent},
    liquidation_calcs::{self, SystemMode},
    types::{
        BorrowStablecoinEffects, DepositAndBorrowEffects, DepositCollateralEffects,
        LiquidationEffects, RepayLoanEffects, WithdrawCollateralEffects,
    },
};

pub fn initialize_borrowing_market(
    market: &mut BorrowingMarketState,
    redemption_bootstrap_ts: u64,
) {
    market.version = 0;
    market.stablecoin_borrowed = 0;
    market.deposited_collateral = CollateralAmounts::default();
    market.base_rate_bps = 0;
    market.num_users = 0;
    market.num_active_users = 0;
    market.bootstrap_period_timestamp = redemption_bootstrap_ts;
}

pub fn approve_trove(
    market: &mut BorrowingMarketState,
    user: &mut UserMetadata,
) -> Result<(), BorrowError> {
    // Initialize trove
    let user_id = market.num_users;

    user.version = 0;
    user.user_id = user_id;

    user.borrowed_stablecoin = 0;
    user.deposited_collateral = CollateralAmounts::default();

    // We only set it to active once we have more than 0 borrowed amount
    user.status = UserStatus::Inactive as u8;

    // Update snapshots
    redistribution::update_user_stake_and_total_stakes(market, user);
    redistribution::update_reward_snapshots(market, user);

    // Update global state
    market.num_users = market.num_users.checked_add(1).unwrap();

    Ok(())
}

pub fn deposit_collateral(
    market: &mut BorrowingMarketState,
    user: &mut UserMetadata,
    amount: u64,
    asset: CollateralToken,
) -> Result<DepositCollateralEffects, crate::BorrowError> {
    assert_not_zero(amount, BorrowError::CannotDepositZeroAmount)?;
    apply_pending_rewards(market, user)?;

    use utils::CollateralStatus::*;
    match UserStatus::from_u8(user.status) {
        Some(UserStatus::Active) => {
            utils::deposit_collateral(market, user, amount, asset, Deposited)
        }
        Some(UserStatus::Inactive) => {
            utils::deposit_collateral(market, user, amount, asset, Inactive)
        }
        _ => unreachable!(),
    }

    redistribution::update_user_stake_and_total_stakes(market, user);

    Ok(DepositCollateralEffects {
        collateral_to_transfer_from_user: CollateralAmounts::of_token(amount, asset),
    })
}

pub fn borrow_stablecoin(
    market: &mut BorrowingMarketState,
    user: &mut UserMetadata,
    staking_pool: &mut StakingPoolState,
    requested_borrow_amount: u64,
    prices: &TokenPrices,
    now: u64,
) -> Result<BorrowStablecoinEffects, crate::BorrowError> {
    assert_not_zero(requested_borrow_amount, BorrowError::CannotBorrowZeroAmount)?;

    let old_debt = user.borrowed_stablecoin;
    apply_pending_rewards(market, user)?;

    let (mode, tcr) = liquidation_calcs::calc_system_mode(
        &market.deposited_collateral,
        market.stablecoin_borrowed,
        prices,
    );

    let fee = match mode {
        SystemMode::Normal => {
            borrowing_rate::refresh_base_rate(market, FeeEvent::Borrowing, now)?;
            borrowing_rate::calc_borrowing_fee(market.base_rate_bps)
        }
        SystemMode::Recovery => 0,
    };

    let borrow_and_fee = BorrowSplit::split_fees(requested_borrow_amount, fee);
    liquidation_calcs::try_borrow(
        borrow_and_fee.amount_to_borrow,
        &market.deposited_collateral,
        market.stablecoin_borrowed,
        &user.deposited_collateral,
        user.borrowed_stablecoin,
        &user.inactive_collateral,
        prices,
        mode,
        tcr,
    )?;

    msg!("Borrowed {:?}", borrow_and_fee);
    let new_debt = user
        .borrowed_stablecoin
        .checked_add(borrow_and_fee.amount_to_borrow)
        .unwrap();

    if new_debt < BORROW_MIN {
        return Err(BorrowError::CannotBorrowLessThanMinimum);
    }

    // At any borrow event for a user, everything that was 'inactive_collateral'
    // becomes immediately active for the given user, and propagates to the market
    let user_inactive = user.inactive_collateral;
    market.inactive_collateral.sub_assign(&user_inactive);
    market.deposited_collateral.add_assign(&user_inactive);

    user.deposited_collateral.add_assign(&user_inactive);
    user.inactive_collateral = CollateralAmounts::default();

    market
        .stablecoin_borrowed
        .checked_add_assign(borrow_and_fee.amount_to_borrow)?;

    user.borrowed_stablecoin = new_debt;

    redistribution::update_user_stake_and_total_stakes(market, user);
    let (staking_fee, treasury_fee) = staking_pool_operations::split_fees(
        borrow_and_fee.fees_to_pay,
        staking_pool.treasury_fee_rate,
    );
    staking_pool_operations::distribute_fees(staking_pool, staking_fee);

    if old_debt == 0 && new_debt > 0 {
        user.status = UserStatus::Active as u8;
        market.num_active_users += 1;
    }

    Ok(BorrowStablecoinEffects {
        amount_mint_to_user: borrow_and_fee.amount_to_borrow - borrow_and_fee.fees_to_pay,
        amount_mint_to_fees_vault: staking_fee,
        amount_mint_to_treasury_vault: treasury_fee,
    })
}

pub fn repay_loan(
    market: &mut BorrowingMarketState,
    user: &mut UserMetadata,
    amount: u64,
) -> Result<RepayLoanEffects, crate::BorrowError> {
    // If there was a redistribution event, update user's balance
    apply_pending_rewards(market, user)?;
    assert_not_zero(user.borrowed_stablecoin, BorrowError::NothingToRepay)?;
    assert_not_zero(amount, BorrowError::CannotRepayZeroAmount)?;

    let payment_amount = u64::min(user.borrowed_stablecoin, amount);
    market.stablecoin_borrowed = market
        .stablecoin_borrowed
        .checked_sub(payment_amount)
        .unwrap();

    let updated_stablecoin_borrowed = user
        .borrowed_stablecoin
        .checked_sub(payment_amount)
        .unwrap();

    if updated_stablecoin_borrowed > 0 && updated_stablecoin_borrowed < BORROW_MIN {
        return Err(BorrowError::TooLowDebt);
    }
    user.borrowed_stablecoin = updated_stablecoin_borrowed;

    redistribution::update_user_stake_and_total_stakes(market, user);

    if updated_stablecoin_borrowed == 0 {
        // If after repayment it's empty, then we don't consider this an active loan
        user.status = UserStatus::Inactive as u8;
        market.num_active_users -= 1;

        // Turn collateral to inactive
        market
            .deposited_collateral
            .sub_assign(&user.deposited_collateral);
        market
            .inactive_collateral
            .add_assign(&user.deposited_collateral);
        user.inactive_collateral
            .add_assign(&user.deposited_collateral);
        user.deposited_collateral = CollateralAmounts::default();
    }

    Ok(RepayLoanEffects {
        amount_to_burn: payment_amount,
        amount_to_transfer: payment_amount,
    })
}

pub fn withdraw_collateral(
    market: &mut BorrowingMarketState,
    user: &mut UserMetadata,
    requested_amount: u64,
    asset: CollateralToken,
    prices: &TokenPrices,
) -> Result<WithdrawCollateralEffects, crate::BorrowError> {
    assert_not_zero(requested_amount, BorrowError::CannotWithdrawZeroAmount)?;
    apply_pending_rewards(market, user)?;

    let user_inactive_token = user.inactive_collateral.token_amount(asset);
    let (withdrawing_active_amt, withdrawing_active, withdrawing_inactive) =
        if user_inactive_token >= requested_amount {
            (
                0,
                CollateralAmounts::default(),
                CollateralAmounts::of_token(requested_amount, asset),
            )
        } else {
            let amt = requested_amount - user_inactive_token;
            (
                amt,
                CollateralAmounts::of_token(amt, asset),
                user.inactive_collateral,
            )
        };

    if withdrawing_active_amt > 0 {
        liquidation_calcs::try_withdraw(
            &withdrawing_active,
            &market.deposited_collateral,
            market.stablecoin_borrowed,
            &user.deposited_collateral,
            user.borrowed_stablecoin,
            prices,
        )?;
    }

    market.inactive_collateral.sub_assign(&withdrawing_inactive);
    market.deposited_collateral.sub_assign(&withdrawing_active);

    user.inactive_collateral.sub_assign(&withdrawing_inactive);
    user.deposited_collateral.sub_assign(&withdrawing_active);

    redistribution::update_user_stake_and_total_stakes(market, user);

    Ok(WithdrawCollateralEffects {
        collateral_to_transfer_to_user: CollateralAmounts::of_token(requested_amount, asset),
        close_user_metadata: user.inactive_collateral.is_zero()
            && user.deposited_collateral.is_zero()
            && user.borrowed_stablecoin == 0,
    })
}

pub fn deposit_and_borrow(
    market: &mut BorrowingMarketState,
    user: &mut UserMetadata,
    staking_pool: &mut StakingPoolState,
    borrow: u64,
    deposit: u64,
    deposit_asset: CollateralToken,
    prices: &TokenPrices,
    now: u64,
) -> Result<DepositAndBorrowEffects, crate::BorrowError> {
    // This instruction allows for an atomic deposit & borrow
    // which would happen when opening a position or when
    // adjusting a position upwards during Recovery mode

    match (borrow, deposit) {
        (0, 0) => Err(BorrowError::CannotDepositZeroAmount),
        (0, _) => Ok(deposit_collateral(market, user, deposit, deposit_asset)?.into()),
        (_, 0) => Ok(borrow_stablecoin(market, user, staking_pool, borrow, prices, now)?.into()),
        (_, _) => {
            // First, deposit inactive collateral
            apply_pending_rewards(market, user)?;

            use utils::CollateralStatus::*;
            utils::deposit_collateral(market, user, deposit, deposit_asset, Inactive);

            // Secondly, borrow some stablecoin
            let BorrowStablecoinEffects {
                amount_mint_to_user,
                amount_mint_to_fees_vault,
                amount_mint_to_treasury_vault,
            } = borrow_stablecoin(market, user, staking_pool, borrow, prices, now)?;

            Ok(DepositAndBorrowEffects {
                amount_mint_to_user,
                amount_mint_to_fees_vault,
                amount_mint_to_treasury_vault,
                collateral_to_transfer_from_user: CollateralAmounts::of_token(
                    deposit,
                    deposit_asset,
                ),
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn try_liquidate(
    liquidator: Pubkey,
    market: &mut BorrowingMarketState,
    user: &mut UserMetadata,
    stability_pool_state: &mut StabilityPoolState,
    epoch_to_scale_to_sum: &mut EpochToScaleToSum,
    token_prices: &TokenPrices,
    liquidations_queue: &mut RefMut<LiquidationsQueue>,
    now_timestamp: u64,
) -> Result<LiquidationEffects, crate::BorrowError> {
    let liquidation_amounts =
        liquidation::liquidate_user(market, user, stability_pool_state, token_prices)?;

    if liquidation_amounts.usd_debt_to_stability_pool > 0 {
        stability_pool_operations::liquidate(
            stability_pool_state,
            epoch_to_scale_to_sum,
            liquidation_amounts.coll_to_stability_pool,
            liquidation_amounts.usd_debt_to_stability_pool,
            now_timestamp,
        )?;
    }

    if liquidation_amounts.usd_debt_to_redistribute > 0 {
        redistribution::redistribute(
            market,
            liquidation_amounts.usd_debt_to_redistribute,
            liquidation_amounts.coll_to_redistribute,
        )?;
    }

    liquidation::update_system_snapshots_after_liquidation(market, user.borrowed_stablecoin);

    let liquidation_event = LiquidationEvent::new(
        liquidator,
        liquidation_amounts.coll_to_liquidator,
        liquidation_amounts.coll_to_clearer,
        liquidation_amounts.coll_to_stability_pool,
        now_timestamp,
    );
    liquidations_queue::add_liquidation_event(liquidation_event, liquidations_queue)?;

    Ok(LiquidationEffects {
        liquidation_event,
        usd_to_burn_from_stability_pool: liquidation_amounts.usd_debt_to_stability_pool,
    })
}

#[cfg(test)]
pub fn refresh_positions(
    market: &mut BorrowingMarketState,
    user: &mut UserMetadata,
) -> Result<(), crate::BorrowError> {
    apply_pending_rewards(market, user)?;
    redistribution::update_user_stake_and_total_stakes(market, user);
    Ok(())
}

pub mod utils {
    use crate::state::CollateralToken;

    use super::{BorrowingMarketState, CollateralAmounts, Pubkey, UserMetadata};
    pub enum CollateralStatus {
        Inactive,
        Deposited,
    }

    pub fn deposit_collateral(
        market: &mut BorrowingMarketState,
        user: &mut UserMetadata,
        amount: u64,
        asset: CollateralToken,
        collateral_status: CollateralStatus,
    ) {
        let deposit = CollateralAmounts::of_token(amount, asset);
        match collateral_status {
            CollateralStatus::Inactive => {
                user.inactive_collateral.add_assign(&deposit);
                market.inactive_collateral.add_assign(&deposit)
            }
            CollateralStatus::Deposited => {
                user.deposited_collateral.add_assign(&deposit);
                market.deposited_collateral.add_assign(&deposit)
            }
        }
    }

    pub fn set_addresses(user: &mut UserMetadata, owner: Pubkey, metadata: Pubkey) {
        user.owner = owner;
        user.metadata_pk = metadata;
    }

    pub fn assert_not_zero(value: u64, err: crate::BorrowError) -> Result<(), crate::BorrowError> {
        if value == 0 {
            Err(err)
        } else {
            Ok(())
        }
    }
}

pub mod redistribution {
    use super::UserBalances;
    use crate::utils::coretypes::CheckedAssign;
    use crate::{state::UserStatus, UserMetadata};
    use crate::{utils::consts::DECIMAL_PRECISION, BorrowingMarketState, CollateralAmounts};

    pub fn redistribute(
        market: &mut BorrowingMarketState,
        stablecoin_debt: u64,
        collateral: CollateralAmounts,
    ) -> Result<(), crate::BorrowError> {
        let total_stake = market.total_stake;

        // TODO: add redistribution precision last error loss
        // println!(
        //     "Redistributing debt {:?} stake {:?} ",
        //     stablecoin_debt, total_stake
        // );
        // println!(
        //     "Redistributing col {:?} stake {:?} ",
        //     collateral.sol, total_stake
        // );

        let coll_reward_per_token = collateral
            .to_token_map()
            .mul_fraction(DECIMAL_PRECISION, total_stake as u128);

        // println!("Extra Coll RPT {}", coll_reward_per_token.sol);

        let stablecoin_reward_per_token = (stablecoin_debt as u128)
            .checked_mul(DECIMAL_PRECISION)
            .unwrap()
            .checked_div(total_stake as u128)
            .unwrap();

        market
            .collateral_reward_per_token
            .add_assign(&coll_reward_per_token);

        market
            .stablecoin_reward_per_token
            .checked_add_assign(stablecoin_reward_per_token)?;

        Ok(())
    }

    pub fn apply_pending_rewards(
        market: &BorrowingMarketState,
        user: &mut UserMetadata,
    ) -> Result<(), crate::BorrowError> {
        if !has_pending_rewards(market, user) {
            return Ok(());
        }

        let pending_sol_reward = get_pending_redistributed_collateral_reward(market, user);
        let pending_stablecoin_reward = get_pending_redistributed_stablecoin_reward(market, user);

        let updated_deposited_collateral = user.deposited_collateral.add(&pending_sol_reward);
        let updated_stablecoin_borrowed = user
            .borrowed_stablecoin
            .checked_add(pending_stablecoin_reward)
            .unwrap();

        user.deposited_collateral = updated_deposited_collateral;
        user.borrowed_stablecoin = updated_stablecoin_borrowed;

        // TOOD: check if need to update user_stake
        update_reward_snapshots(market, user);

        Ok(())
    }

    pub fn get_user_balances(market: &BorrowingMarketState, user: &UserMetadata) -> UserBalances {
        let user_current_debt = user.borrowed_stablecoin;
        let user_pending_debt = get_pending_redistributed_stablecoin_reward(market, user);
        let user_current_collateral = user.deposited_collateral;
        let user_pending_collateral = get_pending_redistributed_collateral_reward(market, user);

        UserBalances {
            user_current_debt,
            user_pending_debt,
            user_current_collateral,
            user_pending_collateral,
        }
    }

    pub fn get_pending_redistributed_stablecoin_reward(
        market: &BorrowingMarketState,
        user: &UserMetadata,
    ) -> u64 {
        let snapshot_stablecoin_reward_per_token = user.user_stablecoin_reward_per_token;

        let latest_stablecoin_reward_per_token = market.stablecoin_reward_per_token;
        let diff_stablecoin_reward_per_token =
            latest_stablecoin_reward_per_token - snapshot_stablecoin_reward_per_token;

        if diff_stablecoin_reward_per_token == 0 || user.status != (UserStatus::Active as u8) {
            return 0;
        }

        let stake = user.user_stake as u128;
        let pending_gain = stake
            .checked_mul(diff_stablecoin_reward_per_token)
            .unwrap()
            .checked_div(DECIMAL_PRECISION)
            .unwrap();

        pending_gain as u64
    }

    pub fn get_pending_redistributed_collateral_reward(
        market: &BorrowingMarketState,
        user: &UserMetadata,
    ) -> CollateralAmounts {
        // rpt = reward per token
        let snapshot_coll_rpt = user.user_collateral_reward_per_token;
        let latest_coll_rpt = market.collateral_reward_per_token;
        let diff_coll_rpt = latest_coll_rpt.sub(&snapshot_coll_rpt);

        if diff_coll_rpt.is_zero() || user.status != (UserStatus::Active as u8) {
            return CollateralAmounts::default();
        }

        diff_coll_rpt
            .mul_fraction(user.user_stake as u128, DECIMAL_PRECISION)
            .to_collateral_amounts()
    }

    pub fn has_pending_rewards(market: &BorrowingMarketState, user: &mut UserMetadata) -> bool {
        // A user has pending rewards if its snapshot is less than the current rewards per-unit-staked sum:
        // this indicates that rewards have occured since the snapshot was made, and the user therefore has
        // pending rewards

        if user.status != (UserStatus::Active as u8) {
            false
        } else {
            user.user_stablecoin_reward_per_token < market.stablecoin_reward_per_token
        }
    }

    pub fn update_reward_snapshots(market: &BorrowingMarketState, user: &mut UserMetadata) {
        user.user_collateral_reward_per_token = market.collateral_reward_per_token;
        user.user_stablecoin_reward_per_token = market.stablecoin_reward_per_token;
    }

    pub fn compute_new_stake(market: &BorrowingMarketState, debt: u64) -> u64 {
        // https://github.com/liquity/dev/blob/9bd735e872f9eb7c7c240151bc81855cc2204499/README.md#redistributions-and-corrected-stakes
        if market.borrowed_stablecoin_snapshot == 0 {
            debt
        } else {
            (debt as u128)
                .checked_mul(market.total_stake_snapshot as u128)
                .unwrap()
                .checked_div(market.borrowed_stablecoin_snapshot as u128)
                .unwrap() as u64
        }
    }

    pub fn update_user_stake_and_total_stakes(
        market: &mut BorrowingMarketState,
        user: &mut UserMetadata,
    ) {
        // Update borrower's stake based on their latest collateral value
        let new_stake = compute_new_stake(market, user.borrowed_stablecoin);
        let old_stake = user.user_stake;
        user.user_stake = new_stake;

        market.total_stake = market
            .total_stake
            .checked_sub(old_stake)
            .unwrap()
            .checked_add(new_stake)
            .unwrap();
    }

    pub fn remove_stake(market: &mut BorrowingMarketState, user: &mut UserMetadata) {
        let stake = user.user_stake;
        market.total_stake = market.total_stake.checked_sub(stake).unwrap();
        user.user_stake = 0;
    }
}

mod liquidation {

    use anchor_lang::prelude::msg;

    use crate::{
        borrowing_market::liquidation_calcs::{self},
        state::UserStatus,
        BorrowError, BorrowingMarketState, CollateralAmounts, StabilityPoolState, TokenPrices,
        UserMetadata,
    };

    use super::{
        redistribution::{self, update_user_stake_and_total_stakes},
        LiquidationBreakdownAmounts, UserBalances,
    };

    fn calculate_liquidation_effects(
        market: &BorrowingMarketState,
        user: &UserMetadata,
        stability_pool_state: &StabilityPoolState,
        prices: &TokenPrices,
    ) -> Result<(UserBalances, LiquidationBreakdownAmounts), crate::BorrowError> {
        // apply pending redistribution amounts
        let user_balances = redistribution::get_user_balances(market, user);

        let total_user_debt = user_balances
            .user_current_debt
            .checked_add(user_balances.user_pending_debt)
            .unwrap();

        let total_user_collateral = user_balances
            .user_current_collateral
            .add(&user_balances.user_pending_collateral);

        let liquidation_breakdown = liquidation_calcs::calculate_liquidation_effects(
            total_user_debt,
            &total_user_collateral,
            market.stablecoin_borrowed,
            &market.deposited_collateral,
            stability_pool_state.stablecoin_deposited,
            prices,
        )?;

        msg!("Liq effects {:?}", liquidation_breakdown);

        Ok((user_balances, liquidation_breakdown))
    }

    pub fn liquidate_user(
        market: &mut BorrowingMarketState,
        user: &mut UserMetadata,
        stability_pool_state: &StabilityPoolState,
        token_prices: &TokenPrices,
    ) -> Result<LiquidationBreakdownAmounts, crate::BorrowError> {
        if market.num_active_users <= 1 {
            msg!("Last user, cannot liquidate the last user");
            return Err(BorrowError::LastUser);
        }

        let (user_balances, liquidation_amounts) =
            calculate_liquidation_effects(market, user, stability_pool_state, token_prices)?;

        let remaining_user_coll = user
            .deposited_collateral
            .add(&user_balances.user_pending_collateral)
            .sub(&liquidation_amounts.coll_to_stability_pool)
            .sub(&liquidation_amounts.coll_to_redistribute)
            .sub(&liquidation_amounts.coll_to_clearer)
            .sub(&liquidation_amounts.coll_to_liquidator);

        market.stablecoin_borrowed = market
            .stablecoin_borrowed
            .checked_sub(liquidation_amounts.usd_debt_to_stability_pool)
            .unwrap();

        market.deposited_collateral = market
            .deposited_collateral
            .sub(&liquidation_amounts.coll_to_stability_pool)
            .sub(&liquidation_amounts.coll_to_clearer)
            .sub(&liquidation_amounts.coll_to_liquidator)
            .sub(&remaining_user_coll);
        market.inactive_collateral.add_assign(&remaining_user_coll);

        // Update user positions
        user.inactive_collateral.add_assign(&remaining_user_coll);
        user.deposited_collateral = CollateralAmounts::default();

        user.borrowed_stablecoin = 0;
        user.status = UserStatus::Inactive as u8;

        market.num_active_users -= 1;

        redistribution::remove_stake(market, user);

        Ok(liquidation_amounts)
    }

    pub fn update_system_snapshots_after_liquidation(market: &mut BorrowingMarketState, debt: u64) {
        // https://github.com/liquity/dev/blob/9bd735e872f9eb7c7c240151bc81855cc2204499/README.md#redistributions-and-corrected-stakes
        market.total_stake_snapshot = market.total_stake;
        market.borrowed_stablecoin_snapshot = market.stablecoin_borrowed;
    }
}
