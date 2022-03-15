use std::cell::RefMut;

use super::liquidations_queue;
use super::types::{
    HarvestLiquidationGainsEffects, ProvideStabilityEffects, WithdrawStabilityEffects,
};
use crate::stability_pool::types::RewardDistributionCalculation;
use crate::state::epoch_to_scale_to_sum::EpochToScaleToSum;
use crate::utils::consts::{DECIMAL_PRECISION, HBB_FACTOR, ONE};
use crate::StabilityTokenMap;
use crate::U256;

#[allow(unused_imports)]
use crate::msg;
use crate::state::StabilityToken;
use crate::{
    fail, BorrowError, CollateralAmounts, LiquidationsQueue, StabilityCollateralAmounts,
    StabilityPoolState, StabilityProviderState,
};
use anchor_lang::prelude::ProgramError;

pub fn initialize_stability_pool(
    stability_pool_state: &mut StabilityPoolState,
    liquidations_queue: &mut RefMut<LiquidationsQueue>,
    hbb_emissions_start_time: u64,
) {
    stability_pool_state.version = 0;
    stability_pool_state.num_users = 0;
    stability_pool_state.stablecoin_deposited = 0;
    stability_pool_state.cumulative_gains_total = StabilityTokenMap::default();
    stability_pool_state.pending_collateral_gains = StabilityTokenMap::default();
    stability_pool_state.current_epoch = 0;
    stability_pool_state.current_scale = 0;
    stability_pool_state.p = ONE;

    stability_pool_state.last_stablecoin_loss_error_offset = 0;
    stability_pool_state.last_coll_loss_error_offset = StabilityCollateralAmounts::default();

    stability_pool_state.hbb_emissions_start_ts = hbb_emissions_start_time;

    liquidations_queue::initialize_queue(liquidations_queue);
}

pub fn approve_new_user(
    stability_pool_state: &mut StabilityPoolState,
    stability_provider_state: &mut StabilityProviderState,
) {
    stability_provider_state.approve_stability(stability_pool_state.num_users);
    stability_pool_state.num_users += 1;
}

pub fn provide_stability(
    stability_pool_state: &mut StabilityPoolState,
    stability_provider_state: &mut StabilityProviderState,
    epoch_to_scale_to_sum: &mut EpochToScaleToSum,
    amount: u64,
    now_timestamp: u64,
) -> Result<ProvideStabilityEffects, ProgramError> {
    trigger_hbb_issuance(stability_pool_state, epoch_to_scale_to_sum, now_timestamp)?;

    // 1. Calculate compounded usd deposit
    let compounded_usd_deposit = liquidations_logic::get_compounded_usd_deposit(
        stability_pool_state,
        stability_provider_state,
    );

    // 2. Recalculate pending state
    liquidations_logic::update_pending_gains(stability_provider_state, epoch_to_scale_to_sum)?;

    // 3. Transfer usd to stability pool
    liquidations_logic::send_usd_to_stability_pool(stability_pool_state, amount)?;

    // 4. Update user deposit and snapshot
    let new_user_usd_deposits = compounded_usd_deposit.checked_add(amount).unwrap();
    stability_provider_state.deposited_stablecoin = new_user_usd_deposits;
    stability_provider_state.user_deposit_snapshot = liquidations_logic::get_new_user_snapshot(
        stability_pool_state,
        epoch_to_scale_to_sum,
        new_user_usd_deposits,
    );

    if compounded_usd_deposit == 0 {
        stability_pool_state.total_users_providing_stability += 1;
    }

    Ok(ProvideStabilityEffects {
        usd_to_stability_pool_transfer: amount,
    })
}

pub fn withdraw_stability(
    stability_pool_state: &mut StabilityPoolState,
    stability_provider_state: &mut StabilityProviderState,
    epoch_to_scale_to_sum: &mut EpochToScaleToSum,
    amount: u64,
    now_timestamp: u64,
) -> Result<WithdrawStabilityEffects, ProgramError> {
    trigger_hbb_issuance(stability_pool_state, epoch_to_scale_to_sum, now_timestamp)?;

    // 1. Calculate compounded usd deposit
    let compounded_usd_deposit = liquidations_logic::get_compounded_usd_deposit(
        stability_pool_state,
        stability_provider_state,
    );

    // 2. Recalculate pending state
    liquidations_logic::update_pending_gains(stability_provider_state, epoch_to_scale_to_sum)?;

    // 3. Send usd back to depositor
    let usd_to_withdraw = u64::min(compounded_usd_deposit, amount);
    liquidations_logic::send_usd_to_depositor(stability_pool_state, usd_to_withdraw)?;

    // 4. Update user deposit and snapshot
    let new_user_usd_deposits = compounded_usd_deposit.checked_sub(amount).unwrap();
    stability_provider_state.deposited_stablecoin = new_user_usd_deposits;
    stability_provider_state.user_deposit_snapshot = liquidations_logic::get_new_user_snapshot(
        stability_pool_state,
        epoch_to_scale_to_sum,
        new_user_usd_deposits,
    );

    if new_user_usd_deposits == 0 {
        stability_pool_state.total_users_providing_stability -= 1;
    }

    Ok(WithdrawStabilityEffects {
        usd_remaining_to_withdraw: usd_to_withdraw as u64,
    })
}

pub fn update_pending_gains(
    stability_pool_state: &mut StabilityPoolState,
    stability_provider_state: &mut StabilityProviderState,
    epoch_to_scale_to_sum: &EpochToScaleToSum,
) -> Result<HarvestLiquidationGainsEffects, ProgramError> {
    liquidations_logic::update_pending_gains(stability_provider_state, epoch_to_scale_to_sum)?;

    let compounded_usd_deposit = liquidations_logic::get_compounded_usd_deposit(
        stability_pool_state,
        stability_provider_state,
    );

    // Update user deposit and snapshot
    let new_user_usd_deposits = compounded_usd_deposit;
    stability_provider_state.deposited_stablecoin = new_user_usd_deposits;

    stability_provider_state.user_deposit_snapshot = liquidations_logic::get_new_user_snapshot(
        stability_pool_state,
        epoch_to_scale_to_sum,
        new_user_usd_deposits,
    );

    let gains = stability_provider_state.pending_gains_per_user;

    Ok(HarvestLiquidationGainsEffects { gains })
}

pub fn harvest_liquidation_gains(
    stability_pool_state: &mut StabilityPoolState,
    stability_provider_state: &mut StabilityProviderState,
    epoch_to_scale_to_sum: &mut EpochToScaleToSum,
    liquidations_queue: &mut RefMut<LiquidationsQueue>,
    now_timestamp: u64,
    harvest_token: StabilityToken,
) -> Result<HarvestLiquidationGainsEffects, ProgramError> {
    if liquidations_queue::has_pending_liquidation_events(liquidations_queue) {
        // if there are outstanding liquidation events
        // that haven't been cleared yet,
        // it means that the reward vaults don't contain all the
        // collateral that they should, and therefore we can't withdraw from
        // them yet
        return Err(BorrowError::CannotHarvestUntilLiquidationGainsCleared.into());
    }

    trigger_hbb_issuance(stability_pool_state, epoch_to_scale_to_sum, now_timestamp)?;

    // Recalculate pending state
    let result = update_pending_gains(
        stability_pool_state,
        stability_provider_state,
        epoch_to_scale_to_sum,
    )?;

    // Update state
    harvest_pending_gains(
        stability_pool_state,
        stability_provider_state,
        harvest_token,
    )?;

    Ok(result)
}

pub fn harvest_pending_gains(
    stability_pool_state: &mut StabilityPoolState,
    stability_provider_state: &mut StabilityProviderState,
    harvest_token: StabilityToken,
) -> Result<(), crate::BorrowError> {
    // harvest_pending_gains only harvests what's already in the state,
    // whereas harvest_liquidation_gains recalculates pending gains
    // and then harvests
    let token_gain_amount = stability_provider_state
        .pending_gains_per_user
        .token_amount(harvest_token);
    let mut gains_to_harvest =
        StabilityCollateralAmounts::of_token(token_gain_amount, harvest_token);
    // always harvest HBB
    gains_to_harvest.hbb = stability_provider_state.pending_gains_per_user.hbb;

    stability_pool_state.pending_collateral_gains = stability_pool_state
        .pending_collateral_gains
        .sub(&gains_to_harvest.to_token_map());
    stability_provider_state.cumulative_gains_per_user = stability_provider_state
        .cumulative_gains_per_user
        .add(&gains_to_harvest.to_token_map());
    stability_provider_state.pending_gains_per_user = stability_provider_state
        .pending_gains_per_user
        .sub(&gains_to_harvest);

    Ok(())
}

pub fn liquidate(
    stability_pool_state: &mut StabilityPoolState,
    epoch_to_scale_to_sum: &mut EpochToScaleToSum,
    collateral_gain_to_stability_pool: CollateralAmounts,
    debt_to_offset: u64,
    now_timestamp: u64,
) -> Result<(), crate::BorrowError> {
    if stability_pool_state.stablecoin_deposited == 0 {
        fail!(BorrowError::StabilityPoolIsEmpty);
    }

    if stability_pool_state.stablecoin_deposited < debt_to_offset {
        fail!(BorrowError::NotEnoughStabilityInTheStabilityPool);
    }

    let hbb_emission = issuance_logic::compute_new_hbb_issuance(
        stability_pool_state.cumulative_gains_total.hbb as u64,
        stability_pool_state.hbb_emissions_start_ts,
        now_timestamp,
    );

    let collateral_gain_to_stability_pool = StabilityCollateralAmounts::new(
        collateral_gain_to_stability_pool.sol,
        collateral_gain_to_stability_pool.eth,
        collateral_gain_to_stability_pool.btc,
        collateral_gain_to_stability_pool.srm,
        collateral_gain_to_stability_pool.ray,
        collateral_gain_to_stability_pool.ftt,
        hbb_emission,
    );

    add_rewards_and_loss(
        stability_pool_state,
        epoch_to_scale_to_sum,
        collateral_gain_to_stability_pool,
        debt_to_offset,
    )?;

    Ok(())
}

fn add_rewards_and_loss(
    stability_pool_state: &mut StabilityPoolState,
    epoch_to_scale_to_sum: &mut EpochToScaleToSum,
    gains: StabilityCollateralAmounts,
    usd_loss: u64,
) -> Result<(), crate::BorrowError> {
    // TODO: should we actually mint/transfer the actual_gains_considering_precision_loss
    // rather than gains
    let RewardDistributionCalculation {
        actual_gains_considering_precision_loss,
        coll_gained_per_unit_staked,
        usd_loss_per_unit_staked,
        last_coll_error,
        last_usd_error,
    } = liquidations_logic::compute_rewards_per_unit_staked(
        stability_pool_state,
        gains,
        usd_loss,
        stability_pool_state.stablecoin_deposited,
    );

    liquidations_logic::update_reward_sum_and_product(
        stability_pool_state,
        epoch_to_scale_to_sum,
        coll_gained_per_unit_staked,
        usd_loss_per_unit_staked,
    )?;

    stability_pool_state.cumulative_gains_total = stability_pool_state
        .cumulative_gains_total
        .add(&actual_gains_considering_precision_loss.to_token_map());
    stability_pool_state.pending_collateral_gains = stability_pool_state
        .pending_collateral_gains
        .add(&actual_gains_considering_precision_loss.to_token_map());

    stability_pool_state.last_stablecoin_loss_error_offset = last_usd_error;
    stability_pool_state.last_coll_loss_error_offset = last_coll_error;
    stability_pool_state.stablecoin_deposited = stability_pool_state
        .stablecoin_deposited
        .checked_sub(
            usd_loss
                + last_usd_error
                    .checked_div(DECIMAL_PRECISION as u64)
                    .unwrap(),
        )
        .unwrap();

    Ok(())
}

fn trigger_hbb_issuance(
    stability_pool_state: &mut StabilityPoolState,
    epoch_to_scale_to_sum: &mut EpochToScaleToSum,
    now_timestamp: u64,
) -> Result<(), crate::BorrowError> {
    if stability_pool_state.stablecoin_deposited == 0 {
        return Ok(());
    }

    let hbb_emission = issuance_logic::compute_new_hbb_issuance(
        stability_pool_state.cumulative_gains_total.hbb as u64,
        stability_pool_state.hbb_emissions_start_ts,
        now_timestamp,
    );

    add_rewards_and_loss(
        stability_pool_state,
        epoch_to_scale_to_sum,
        StabilityCollateralAmounts {
            hbb: hbb_emission,
            ..Default::default()
        },
        0,
    )?;

    Ok(())
}

mod liquidations_logic {

    use crate::{
        stability_pool::types::RewardDistributionCalculation,
        utils::consts::{DECIMAL_PRECISION, ONE, SCALE_FACTOR},
        BorrowError, DepositSnapshot,
    };

    use super::*;
    pub fn send_usd_to_stability_pool(
        stability_pool_state: &mut StabilityPoolState,
        amount: u64,
    ) -> Result<(), crate::BorrowError> {
        // Transfer to the pool

        if amount == 0 {
            return Err(BorrowError::CannotProvideZeroStability);
        }

        stability_pool_state.stablecoin_deposited = stability_pool_state
            .stablecoin_deposited
            .checked_add(amount)
            .unwrap();

        Ok(())
    }

    pub fn send_usd_to_depositor(
        stability_pool_state: &mut StabilityPoolState,
        amount: u64,
    ) -> Result<(), crate::BorrowError> {
        if amount == 0 {
            return Err(BorrowError::NothingToUnstake);
        }

        stability_pool_state.stablecoin_deposited = stability_pool_state
            .stablecoin_deposited
            .checked_sub(amount)
            .unwrap();

        Ok(())
    }

    pub fn update_pending_gains(
        stability_provider_state: &mut StabilityProviderState,
        epoch_to_scale_to_sum: &EpochToScaleToSum,
    ) -> Result<(), crate::BorrowError> {
        let pending_gain: StabilityCollateralAmounts =
            get_depositor_pending_gain(stability_provider_state, epoch_to_scale_to_sum);

        let new_pending_gains = stability_provider_state
            .pending_gains_per_user
            .add(&pending_gain);

        stability_provider_state.pending_gains_per_user = new_pending_gains;

        Ok(())
    }

    pub fn get_new_user_snapshot(
        stability_pool_state: &StabilityPoolState,
        epoch_to_scale_to_sum: &EpochToScaleToSum,
        amount: u64,
    ) -> DepositSnapshot {
        if amount == 0 {
            DepositSnapshot::default()
        } else {
            DepositSnapshot::new(
                epoch_to_scale_to_sum
                    .get_sum(
                        stability_pool_state.current_epoch,
                        stability_pool_state.current_scale,
                    )
                    .unwrap(),
                stability_pool_state.p,
                stability_pool_state.current_scale,
                stability_pool_state.current_epoch,
            )
        }
    }

    pub fn get_compounded_usd_deposit(
        stability_pool_state: &StabilityPoolState,
        stability_provider_state: &StabilityProviderState,
    ) -> u64 {
        let stability_pool_state_p = stability_pool_state.p;
        let stability_pool_state_current_scale = stability_pool_state.current_scale;
        let stability_pool_state_current_epoch = stability_pool_state.current_epoch;

        let initial_deposit = stability_provider_state.deposited_stablecoin;
        let deposit_snapshot = &stability_provider_state.user_deposit_snapshot;

        if initial_deposit == 0 || !deposit_snapshot.enabled {
            0
        } else {
            get_compounded_stake_from_snapshots(
                stability_pool_state_p,
                stability_pool_state_current_scale,
                stability_pool_state_current_epoch,
                initial_deposit,
                deposit_snapshot,
            )
        }
    }

    pub fn compute_rewards_per_unit_staked(
        stability_pool_state: &mut StabilityPoolState,
        coll_to_add: StabilityCollateralAmounts,
        debt_to_offset: u64,
        total_usd_deposits: u64,
    ) -> RewardDistributionCalculation {
        let (usd_loss_per_unit_staked, last_usd_error) = match debt_to_offset {
            // if full depletion
            x if x == total_usd_deposits => (ONE as u64, 0),
            // if only a reward
            0 => (0, stability_pool_state.last_stablecoin_loss_error_offset),
            debt_to_offset => {
                /*
                Add 1 to make error in quotient positive. We want "slightly too much" USD loss,
                which ensures the error in any given compounded_usd_deposit favors the Stability Pool.
                */
                let usd_loss_numerator = (debt_to_offset as u128)
                    .checked_mul(DECIMAL_PRECISION)
                    .unwrap()
                    .checked_sub(stability_pool_state.last_stablecoin_loss_error_offset as u128)
                    .unwrap();

                let usd_loss_per_unit_staked = usd_loss_numerator
                    .checked_div(total_usd_deposits as u128)
                    .unwrap()
                    .checked_add(1)
                    .unwrap();

                let last_usd_error = usd_loss_per_unit_staked
                    .checked_mul(total_usd_deposits as u128)
                    .unwrap()
                    .checked_sub(usd_loss_numerator)
                    .unwrap();

                (usd_loss_per_unit_staked as u64, last_usd_error as u64)
            }
        };

        let coll_gain_numerator = coll_to_add
            .to_token_map()
            .mul_scalar(DECIMAL_PRECISION)
            .add(
                &stability_pool_state
                    .last_coll_loss_error_offset
                    .to_token_map(),
            );

        let coll_gained_per_unit_staked =
            coll_gain_numerator.div_scalar(total_usd_deposits as u128);
        let last_coll_error = coll_gain_numerator
            .sub(&coll_gained_per_unit_staked.mul_scalar(total_usd_deposits as u128))
            .to_collateral_amounts();

        let actual_issuance = coll_gain_numerator
            .sub(&last_coll_error.to_token_map())
            .div_scalar(DECIMAL_PRECISION)
            .to_collateral_amounts();

        RewardDistributionCalculation {
            actual_gains_considering_precision_loss: actual_issuance,
            coll_gained_per_unit_staked,
            usd_loss_per_unit_staked,
            last_coll_error,
            last_usd_error: last_usd_error as u64,
        }
    }

    pub fn update_reward_sum_and_product(
        stability_pool_state: &mut StabilityPoolState,
        epoch_to_scale_to_sum: &mut EpochToScaleToSum,
        coll_gained_per_unit_staked: StabilityTokenMap,
        usd_loss_per_unit_staked: u64,
    ) -> Result<(), crate::BorrowError> {
        // current status
        let current_p = stability_pool_state.p;
        let current_epoch = stability_pool_state.current_epoch;
        let current_scale = stability_pool_state.current_scale;
        let current_s = &epoch_to_scale_to_sum
            .get_sum(
                stability_pool_state.current_epoch,
                stability_pool_state.current_scale,
            )
            .unwrap();

        // msg!("usd_loss_per_unit_staked {}", usd_loss_per_unit_staked);
        let new_product_factor = ONE.checked_sub(usd_loss_per_unit_staked as u128).unwrap();

        // Calculate the new S first
        let new_sum = coll_gained_per_unit_staked
            .mul_scalar(current_p)
            .add(current_s);

        // Calculate new P
        let (new_p, new_epoch, new_scale) = if new_product_factor == 0 {
            // Stability pool depleted, new epoch
            (ONE, current_epoch + 1, 0)
        } else if current_p
            .checked_mul(new_product_factor)
            .unwrap()
            .checked_div(DECIMAL_PRECISION)
            .unwrap()
            < SCALE_FACTOR
        {
            // P is too small, losing precision, increasing the scale (same epoch)
            (
                current_p
                    .checked_mul(new_product_factor)
                    .unwrap()
                    .checked_div(DECIMAL_PRECISION)
                    .unwrap()
                    .checked_mul(SCALE_FACTOR)
                    .unwrap(),
                current_epoch,
                current_scale + 1,
            )
        } else {
            // most common case, just upgrade P (same epoch, same scale)
            (
                current_p
                    .checked_mul(new_product_factor)
                    .unwrap()
                    .checked_div(DECIMAL_PRECISION)
                    .unwrap(),
                stability_pool_state.current_epoch,
                stability_pool_state.current_scale,
            )
        };

        // Update global states
        epoch_to_scale_to_sum.set_sum(
            stability_pool_state.current_epoch,
            stability_pool_state.current_scale,
            new_sum,
        )?;

        update_stability_pool_snapshot(
            stability_pool_state,
            epoch_to_scale_to_sum,
            new_p,
            new_epoch,
            new_scale,
        )?;

        Ok(())
    }

    fn update_stability_pool_snapshot(
        stability_pool_state: &mut StabilityPoolState,
        epoch_to_scale_to_sum: &mut EpochToScaleToSum,
        new_p: u128,
        new_epoch: u64,
        new_scale: u64,
    ) -> Result<(), crate::BorrowError> {
        let current_epoch = stability_pool_state.current_epoch;
        let current_scale = stability_pool_state.current_scale;

        stability_pool_state.p = new_p;
        stability_pool_state.current_epoch = new_epoch;
        stability_pool_state.current_scale = new_scale;

        if current_epoch != new_epoch || current_scale != new_scale {
            let new_sum = StabilityTokenMap::default();
            epoch_to_scale_to_sum.set_sum(new_epoch, new_scale, new_sum)?;
        }

        Ok(())
    }

    fn get_depositor_pending_gain(
        stability_provider_state: &StabilityProviderState,
        epoch_to_scale_to_sum: &EpochToScaleToSum,
    ) -> StabilityCollateralAmounts {
        let initial_deposit = stability_provider_state.deposited_stablecoin;
        if initial_deposit == 0 {
            StabilityCollateralAmounts::default()
        } else {
            let deposit_snapshot = &stability_provider_state.user_deposit_snapshot;
            get_pending_gain_from_snapshot(initial_deposit, deposit_snapshot, epoch_to_scale_to_sum)
        }
    }

    fn get_pending_gain_from_snapshot(
        initial_deposit: u64,
        deposit_snapshot: &DepositSnapshot,
        epoch_to_scale_to_sum: &EpochToScaleToSum,
    ) -> StabilityCollateralAmounts {
        let epoch_snapshot = deposit_snapshot.epoch;
        let scale_snapshot = deposit_snapshot.scale;

        let s_snapshot = &deposit_snapshot.sum;
        let p_snapshot = deposit_snapshot.product;

        let first_portion = epoch_to_scale_to_sum
            .get_sum(epoch_snapshot, scale_snapshot)
            .unwrap()
            .sub(s_snapshot);

        let second_portion = epoch_to_scale_to_sum
            .get_sum(epoch_snapshot, scale_snapshot + 1)
            .unwrap_or_default()
            .div_scalar(SCALE_FACTOR as u128);

        let res = first_portion.add(&second_portion);

        let StabilityTokenMap {
            sol,
            eth,
            btc,
            srm,
            ray,
            ftt,
            hbb,
        } = res;

        let sol = U256::from(sol)
            .checked_mul(U256::from(initial_deposit))
            .unwrap()
            .checked_div(U256::from(p_snapshot))
            .unwrap()
            .checked_div(U256::from(DECIMAL_PRECISION))
            .unwrap();
        let eth = U256::from(eth)
            .checked_mul(U256::from(initial_deposit))
            .unwrap()
            .checked_div(U256::from(p_snapshot))
            .unwrap()
            .checked_div(U256::from(DECIMAL_PRECISION))
            .unwrap();
        let btc = U256::from(btc)
            .checked_mul(U256::from(initial_deposit))
            .unwrap()
            .checked_div(U256::from(p_snapshot))
            .unwrap()
            .checked_div(U256::from(DECIMAL_PRECISION))
            .unwrap();
        let srm = U256::from(srm)
            .checked_mul(U256::from(initial_deposit))
            .unwrap()
            .checked_div(U256::from(p_snapshot))
            .unwrap()
            .checked_div(U256::from(DECIMAL_PRECISION))
            .unwrap();
        let ray = U256::from(ray)
            .checked_mul(U256::from(initial_deposit))
            .unwrap()
            .checked_div(U256::from(p_snapshot))
            .unwrap()
            .checked_div(U256::from(DECIMAL_PRECISION))
            .unwrap();
        let ftt = U256::from(ftt)
            .checked_mul(U256::from(initial_deposit))
            .unwrap()
            .checked_div(U256::from(p_snapshot))
            .unwrap()
            .checked_div(U256::from(DECIMAL_PRECISION))
            .unwrap();
        let hbb = U256::from(hbb)
            .checked_mul(U256::from(initial_deposit))
            .unwrap()
            .checked_div(U256::from(p_snapshot))
            .unwrap()
            .checked_div(U256::from(DECIMAL_PRECISION))
            .unwrap();

        StabilityCollateralAmounts {
            sol: sol.as_u64(),
            eth: eth.as_u64(),
            btc: btc.as_u64(),
            srm: srm.as_u64(),
            ray: ray.as_u64(),
            ftt: ftt.as_u64(),
            hbb: hbb.as_u64(),
        }
        // .mul(initial_deposit as u128)
        // .div(p_snapshot as u128)
        // .div(DECIMAL_PRECISION);
    }

    fn get_compounded_stake_from_snapshots(
        stability_pool_state_p: u128,
        stability_pool_state_current_scale: u64,
        stability_pool_state_current_epoch: u64,
        initial_stake: u64,
        snapshot: &DepositSnapshot,
    ) -> u64 {
        let snapshot_p = snapshot.product;
        let scale_snapshot = snapshot.scale;
        let epoch_snapshot = snapshot.epoch;

        // shadow them, make them floats so we don't lose precision & overflow
        let stability_pool_state_p = stability_pool_state_p;
        let scale_factor = SCALE_FACTOR as u128;
        let initial_stake = initial_stake;

        if epoch_snapshot < stability_pool_state_current_epoch {
            return 0;
        }

        let scale_diff = stability_pool_state_current_scale - scale_snapshot;

        (match scale_diff {
            0 => (initial_stake as u128)
                .checked_mul(stability_pool_state_p)
                .unwrap()
                .checked_div(snapshot_p)
                .unwrap(),
            1 => (initial_stake as u128)
                .checked_mul(stability_pool_state_p)
                .unwrap()
                .checked_div(snapshot_p)
                .unwrap()
                .checked_div(scale_factor)
                .unwrap(),
            _ => 0,
        }) as u64
    }
}

pub mod issuance_logic {

    use super::HBB_FACTOR;
    use crate::utils::consts::{
        HBB_ISSUANCE_FACTOR, SECONDS_PER_MINUTE, TOTAL_HBB_TO_STABILITY_POOL,
    };

    #[cfg(not(test))]
    use anchor_lang::prelude::msg;
    use decimal_wad::{
        common::{TryMul, TrySub},
        rate::Rate,
    };

    pub fn compute_new_hbb_issuance(
        total_issued_so_far: u64,
        start_issuance_timestamp: u64,
        now_timestamp: u64,
    ) -> u64 {
        let expected_issued_so_far: u64 =
            expected_issuance_since_start(start_issuance_timestamp, now_timestamp);

        let remaining_issuance = expected_issued_so_far
            .checked_sub(total_issued_so_far)
            .unwrap();

        #[cfg(not(test))]
        msg!(
            "Issuing {} HBB as of {} with an existing {}",
            remaining_issuance,
            now_timestamp,
            total_issued_so_far
        );
        remaining_issuance
    }

    pub fn expected_issuance_since_start(start: u64, now: u64) -> u64 {
        // 32,000,000 * (1–0.5^year)
        // halving yearly

        // The float implementation is, but we can't do on chain
        // let factor = 1.0 - HALF.powf(years_diff);

        // The issuance factor F determines the curvature of the issuance curve.
        // Minutes in one year: 60*24*365 = 525600
        // For 50% of remaining tokens issued each year, with minutes as time units, we have:
        // F ** 525600 = 0.5
        //
        // Re-arranging:
        //
        // 525600 * ln(F) = ln(0.5)
        // F = 0.5 ** (1/525600)
        // F = 0.999998681227695000
        //      1000000000000000000 -> decimal precision

        let seconds_diff = now.checked_sub(start).unwrap();
        let minutes_diff = seconds_diff.checked_div(SECONDS_PER_MINUTE).unwrap();

        let one = Rate::one();
        let factor = Rate::from_scaled_val(HBB_ISSUANCE_FACTOR);

        let rate = factor.try_pow(minutes_diff).unwrap();
        let fraction = one.try_sub(rate).unwrap();

        let total_hbb = Rate::from_scaled_val(
            TOTAL_HBB_TO_STABILITY_POOL
                .checked_mul(HBB_FACTOR as u64)
                .unwrap(),
        );
        let issuance = total_hbb.try_mul(fraction).unwrap();

        issuance.to_scaled_val() as u64
    }
}
