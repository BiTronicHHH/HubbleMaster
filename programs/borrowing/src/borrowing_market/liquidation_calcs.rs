#![allow(clippy::just_underscores_and_digits)]
use crate::{
    utils::{
        consts::{CLEARER_RATE, LIQUIDATOR_RATE, NORMAL_MCR, RECOVERY_MCR},
        finance::CollateralInfo,
    },
    BorrowError, CollateralAmounts, TokenPrices,
};

use decimal_wad::{decimal::Decimal, ratio::Ratio};

use super::borrowing_operations::LiquidationBreakdownAmounts;

#[derive(Debug)]
enum LiquidationDecision {
    RedistributeAll,
    StabilityPoolAll,
    StabilityPoolThenRedistribute,
    DoNothing,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SystemMode {
    Normal,
    Recovery,
}

pub struct LiquidationSplit {
    pub collateral_to_liquidate: CollateralAmounts,
    pub collateral_to_liquidator: CollateralAmounts,
    pub collateral_to_clearer: CollateralAmounts,
}

#[derive(Debug)]
pub struct LiquidationDecisionInputs {
    mode: SystemMode,
    mcr: Decimal,
    icr: Decimal,
    tcr: Decimal,
}

#[allow(clippy::too_many_arguments)]
pub fn try_borrow(
    requested_amount: u64,
    global_collateral: &CollateralAmounts,
    global_debt: u64,
    user_collateral: &CollateralAmounts,
    user_debt: u64,
    user_inactive_collateral: &CollateralAmounts,
    prices: &TokenPrices,
    current_mode: SystemMode,
    current_tcr: Decimal,
) -> Result<(), BorrowError> {
    // Any borrow event turns the inactive collateral into
    // backing collateral supporting the loan
    let market_value_usdh = CollateralInfo::calc_market_value_usdh(
        prices,
        &user_collateral.add(user_inactive_collateral),
    );
    let new_debt_usdh = user_debt + requested_amount;
    let new_icr = CollateralInfo::coll_ratio(new_debt_usdh, market_value_usdh);

    let (new_mode, new_tcr) = calc_system_mode(
        &global_collateral.add(user_inactive_collateral),
        global_debt + requested_amount,
        prices,
    );

    let mcr = match current_mode {
        SystemMode::Normal => Decimal::from_percent(NORMAL_MCR),
        SystemMode::Recovery => Decimal::from_percent(RECOVERY_MCR),
    };

    if current_mode == SystemMode::Recovery && new_tcr < current_tcr {
        return Err(BorrowError::OperationLowersSystemTCRInRecoveryMode);
    }

    if current_mode == SystemMode::Normal && new_mode == SystemMode::Recovery {
        return Err(BorrowError::OperationBringsSystemToRecoveryMode);
    }

    if new_icr < mcr {
        return Err(BorrowError::NotEnoughCollateral);
    }

    Ok(())
}

pub fn try_withdraw(
    withdrawing: &CollateralAmounts,
    global_collateral: &CollateralAmounts,
    global_debt: u64,
    user_collateral: &CollateralAmounts,
    user_debt: u64,
    prices: &TokenPrices,
) -> Result<(), BorrowError> {
    // If system is in recovery mode, disallow more withdrawing
    let (mode, _) = calc_system_mode(global_collateral, global_debt, prices);
    if mode == SystemMode::Recovery {
        return Err(BorrowError::CannotWithdrawInRecoveryMode);
    }

    let new_collateral = user_collateral.sub(withdrawing);
    let new_coll_ratio = CollateralInfo::calc_coll_ratio(user_debt, &new_collateral, prices);

    let mcr = Decimal::from_percent(NORMAL_MCR);
    if new_coll_ratio < mcr {
        return Err(BorrowError::NotEnoughCollateral);
    };

    let (new_mode, _) = calc_system_mode(&global_collateral.sub(withdrawing), global_debt, prices);
    if new_mode == SystemMode::Recovery {
        return Err(BorrowError::OperationBringsSystemToRecoveryMode);
    }

    Ok(())
}

pub fn calc_system_mode(
    global_deposited_collateral: &CollateralAmounts,
    global_debt: u64,
    prices: &TokenPrices,
) -> (SystemMode, Decimal) {
    let _150 = Decimal::from_percent(150);
    let tcr = CollateralInfo::calculate_collateral_value(
        global_debt,
        global_deposited_collateral,
        prices,
    )
    .collateral_ratio;
    (
        if tcr < _150 {
            SystemMode::Recovery
        } else {
            SystemMode::Normal
        },
        tcr,
    )
}

fn calc_liq_inputs(
    user_debt: u64,
    user_collateral: &CollateralAmounts,
    global_debt: u64,
    global_collateral: &CollateralAmounts,
    prices: &TokenPrices,
) -> LiquidationDecisionInputs {
    let icr: Decimal = CollateralInfo::calc_coll_ratio(user_debt, user_collateral, prices);
    let mcr: Decimal = Decimal::from_percent(NORMAL_MCR);

    let (mode, tcr): (SystemMode, Decimal) =
        calc_system_mode(global_collateral, global_debt, prices);

    // println!(
    //     "TCR {:?}% ICR {:?}%",
    //     tcr.to_percent().unwrap(),
    //     icr.to_percent().unwrap()
    // );

    LiquidationDecisionInputs {
        mode,
        mcr,
        icr,
        tcr,
    }
}

fn evaluate_liquidation_decision(
    user_debt: u64,
    user_collateral: &CollateralAmounts,
    global_debt: u64,
    global_collateral: &CollateralAmounts,
    usdh_in_sp: u64,
    prices: &TokenPrices,
) -> LiquidationDecision {
    // Firstly we take the fees, then we redistribute and offset
    // with the stability pool. Even if, after fees,
    // the amount is below 100%, these are the terms.
    let _100 = Decimal::from_percent(100);
    let _150 = Decimal::from_percent(150);

    let LiquidationDecisionInputs {
        mode,
        mcr,
        icr,
        tcr,
    } = calc_liq_inputs(
        user_debt,
        user_collateral,
        global_debt,
        global_collateral,
        prices,
    );

    match mode {
        SystemMode::Normal => {
            if icr < mcr {
                LiquidationDecision::StabilityPoolThenRedistribute
            } else {
                LiquidationDecision::DoNothing
            }
        }
        SystemMode::Recovery => {
            if icr <= _100 {
                // user is completely undercollateralized
                // but we cannot send this to the stability pool
                // as it would incur a net loss, so we redistribute
                // among all open debt positions
                LiquidationDecision::RedistributeAll
            } else {
                if icr < mcr {
                    // user is between 100% and 110%
                    // and the stability pool can absorb it all
                    // and takes all the collateral
                    LiquidationDecision::StabilityPoolThenRedistribute
                } else {
                    if icr < tcr {
                        if user_debt <= usdh_in_sp {
                            // user is below 150% and system is in recovery mode
                            // so we liquidate with 10% penalty
                            // the stability pool can absorb the entire debt
                            LiquidationDecision::StabilityPoolAll
                        } else {
                            // redistributing will make everyone else worse off
                            // we might as well leave this position as is (above MCR still)
                            LiquidationDecision::DoNothing
                        }
                    } else {
                        // Position is above TCR (possibly below 150%)
                        // But it's making everyone better off by being above average
                        LiquidationDecision::DoNothing
                    }
                }
            }
        }
    }
}

fn split_stability_and_redistribution(
    usdh_in_sp: u64,
    user_debt: u64,
    user_collateral: &CollateralAmounts,
    liquidation_decision: LiquidationDecision,
    prices: &TokenPrices,
) -> LiquidationBreakdownAmounts {
    // First, calculate ratios
    // anything above 110% remains with the user
    let mv = CollateralInfo::calc_market_value_usdh(prices, &user_collateral);
    let liquidatable_mv = user_debt * 110 / 100;
    let liquidatable_mv = u64::min(liquidatable_mv, mv);
    let liquidatable_coll = user_collateral.mul_fraction(liquidatable_mv, mv);

    // Then, take the fees
    let coll_split = calculate_liquidation_split(&liquidatable_coll, LIQUIDATOR_RATE, CLEARER_RATE);
    let collateral_loss = coll_split.collateral_to_liquidate;

    match liquidation_decision {
        LiquidationDecision::RedistributeAll => LiquidationBreakdownAmounts {
            usd_debt_to_redistribute: user_debt,
            usd_debt_to_stability_pool: 0,
            coll_to_redistribute: coll_split.collateral_to_liquidate,
            coll_to_stability_pool: CollateralAmounts::default(),
            coll_to_liquidator: coll_split.collateral_to_liquidator,
            coll_to_clearer: coll_split.collateral_to_clearer,
        },
        LiquidationDecision::StabilityPoolAll => LiquidationBreakdownAmounts {
            usd_debt_to_redistribute: 0,
            usd_debt_to_stability_pool: user_debt,
            coll_to_redistribute: CollateralAmounts::default(),
            coll_to_stability_pool: collateral_loss,
            coll_to_liquidator: coll_split.collateral_to_liquidator,
            coll_to_clearer: coll_split.collateral_to_clearer,
        },
        LiquidationDecision::StabilityPoolThenRedistribute => {
            // How much can the SP take
            let usd_to_sp_max = u64::min(usdh_in_sp, user_debt);
            let sp_ratio = Ratio::new(usd_to_sp_max, user_debt);
            let usd_to_sp = sp_ratio.mul(user_debt);
            let usd_to_redistribute = user_debt.checked_sub(usd_to_sp).unwrap();
            let coll_to_sp = collateral_loss.mul_fraction(sp_ratio.numerator, sp_ratio.denominator);
            let coll_to_redistribute = collateral_loss.sub(&coll_to_sp);

            LiquidationBreakdownAmounts {
                usd_debt_to_redistribute: usd_to_redistribute,
                usd_debt_to_stability_pool: usd_to_sp,
                coll_to_redistribute,
                coll_to_stability_pool: coll_to_sp,
                coll_to_liquidator: coll_split.collateral_to_liquidator,
                coll_to_clearer: coll_split.collateral_to_clearer,
            }
        }
        LiquidationDecision::DoNothing => unreachable!(),
    }
}

pub fn calculate_liquidation_effects(
    user_debt: u64,
    user_collateral: &CollateralAmounts,
    global_debt: u64,
    global_collateral: &CollateralAmounts,
    usdh_in_sp: u64,
    prices: &TokenPrices,
) -> Result<LiquidationBreakdownAmounts, crate::BorrowError> {
    let liquidation_decision = evaluate_liquidation_decision(
        user_debt,
        user_collateral,
        global_debt,
        global_collateral,
        usdh_in_sp,
        prices,
    );
    match liquidation_decision {
        LiquidationDecision::DoNothing => Err(BorrowError::UserWellCollateralized),
        _ => Ok(split_stability_and_redistribution(
            usdh_in_sp,
            user_debt,
            user_collateral,
            liquidation_decision,
            prices,
        )),
    }
}

fn calculate_liquidation_split(
    collateral_deposited: &CollateralAmounts,
    liquidator_rate_bps: u16,
    clearer_rate_bps: u16,
) -> LiquidationSplit {
    let liquidator_gain = collateral_deposited.mul_bps(liquidator_rate_bps);
    let clearer_gain = collateral_deposited.mul_bps(clearer_rate_bps);
    let coll_gain = collateral_deposited
        .sub(&liquidator_gain)
        .sub(&clearer_gain);
    LiquidationSplit {
        collateral_to_liquidate: coll_gain,
        collateral_to_liquidator: liquidator_gain,
        collateral_to_clearer: clearer_gain,
    }
}
