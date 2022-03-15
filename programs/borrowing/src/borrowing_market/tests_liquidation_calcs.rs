#![allow(unaligned_references)]

use std::cell::RefCell;

use crate::borrowing_market::borrowing_operations::{self, LiquidationBreakdownAmounts};
use crate::borrowing_market::liquidation_calcs;
use crate::stability_pool::stability_pool_operations;
use crate::state::epoch_to_scale_to_sum::EpochToScaleToSum;
use crate::state::{CollateralToken, TokenPrices};
use crate::utils::consts::{CLEARER_RATE, LIQUIDATOR_RATE};
use crate::utils::coretypes::{SOL, USDH};
use crate::utils::finance::CollateralInfo;
use crate::utils::math::coll_to_lamports;
use crate::{
    BorrowError, CollateralAmounts, LiquidationsQueue, StabilityPoolState, StabilityProviderState,
    UserMetadata,
};
use anchor_lang::prelude::Pubkey;
pub use anchor_lang::solana_program::native_token::{lamports_to_sol, sol_to_lamports};

// Tests
// - [x] ICR < 100%  -> all redistribute
// - [x] 100% < ICR < MCR & SP LUSD > Trove debt -> all stability pool
// - [x] 100% < ICR < MCR & SP LUSD < Trove debt -> sp first, redis after
// - [x] MCR <= ICR < 150% & SP LUSD >= Trove debt
//    - [x] if recovery mode -> all stability pool, 110% only
//    - [x] if normal mode -> nothing
// - [x] MCR <= ICR < 150% & SP LUSD < Trove debt -> all stability pool
//    - [x] if recovery mode -> sp first, then redis, 110% only
//    - [x] if normal mode -> nothing
// - [x] ICR >= 150% -> nothing

// Tests
// - normal mode:
//     - [x] new TCR > CCR
//          - [x] open position
//          - [x] add coll
//          - [x] withdraw coll
//          - [x] borrow more
//          - [x] repay
//     - [x] cannot borrow to bring into recovery
//     - [x] cannot withdraw to bring into recovery
//     - [x] active + inactive collateral
// - recovery mode:
//     - [x] can always top up coll
//     - [x] can always repay anything
//     - [x] cannot withdraw collateral
//     - [x] cannot only borrow (without coll topup)
//     - [x] block txns that lower the TCR (only open a new position with > 150%)
//     - [x] when borrow+deposit==atomic
//          - [x] if position adjustment lowers ICR -> block
//          - [x] can (deposit+borrow) to improve personal CR even if still Recovery
//          - [x] how to make an atomic adjustment? should we? (maybe just open a new loan)
//          - [x] if adding more debt -> assert new ICR > CCR
//          - [x] if adding more debt -> assert new ICR > above old ICR
//          - [x] allow borrow+deposit at 0% during recovery mode

fn sol_collateral(amt: f64) -> CollateralAmounts {
    CollateralAmounts::of_token(SOL::from(amt), CollateralToken::SOL)
}

const FEES: u16 = LIQUIDATOR_RATE + CLEARER_RATE;

#[test]
fn test_liquidation_calcs_no_liquidation() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 200% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(2.0);
    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(5.0);
    let usd_in_sp = USDH::from(3.0);

    let res = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    );

    assert_eq!(res.err().unwrap(), BorrowError::UserWellCollateralized);
}

#[test]
fn test_liquidation_calcs_undercollateralized_recovery_mode() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 80% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(0.8);

    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(2.0);
    let usd_in_sp = USDH::from(3.0);

    let LiquidationBreakdownAmounts {
        usd_debt_to_redistribute,
        usd_debt_to_stability_pool,
        coll_to_redistribute,
        coll_to_stability_pool,
        coll_to_liquidator,
        coll_to_clearer,
    } = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    )
    .unwrap();

    // all is redistributed

    assert_eq!(usd_debt_to_stability_pool, 0);
    assert_eq!(usd_debt_to_redistribute, user_debt);
    assert_eq!(coll_to_redistribute, user_collateral.mul_bps(10_000 - FEES));
    assert_eq!(coll_to_stability_pool, CollateralAmounts::default());

    assert_eq!(coll_to_liquidator, user_collateral.mul_bps(LIQUIDATOR_RATE));
    assert_eq!(coll_to_clearer, user_collateral.mul_bps(CLEARER_RATE));
}

#[test]
fn test_liquidation_calcs_undercollateralized_normal_mode() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 80% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(0.8);

    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(5.0);
    let usd_in_sp = USDH::from(0.6);

    let LiquidationBreakdownAmounts {
        usd_debt_to_redistribute,
        usd_debt_to_stability_pool,
        coll_to_redistribute,
        coll_to_stability_pool,
        coll_to_liquidator,
        coll_to_clearer,
    } = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    )
    .unwrap();

    // SP takes first, rest is redistributed

    assert_eq!(usd_debt_to_stability_pool, usd_in_sp);
    assert_eq!(usd_debt_to_redistribute, user_debt - usd_in_sp);
    assert_eq!(
        coll_to_redistribute,
        user_collateral.mul_bps((((10_000 as u16 - FEES) as u64) * 40 / 100) as u16)
    );
    assert_eq!(
        coll_to_stability_pool,
        user_collateral.mul_bps((((10_000 as u16 - FEES) as u64) * 60 / 100) as u16)
    );

    assert_eq!(coll_to_liquidator, user_collateral.mul_bps(LIQUIDATOR_RATE));
    assert_eq!(coll_to_clearer, user_collateral.mul_bps(CLEARER_RATE));
}

#[test]
fn test_liquidation_calcs_between_100_and_110_all_goes_to_sp() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 105% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(1.05);

    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(5.0);
    let usd_in_sp = USDH::from(3.0);

    let LiquidationBreakdownAmounts {
        usd_debt_to_redistribute,
        usd_debt_to_stability_pool,
        coll_to_redistribute,
        coll_to_stability_pool,
        coll_to_liquidator,
        coll_to_clearer,
    } = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    )
    .unwrap();

    assert_eq!(usd_debt_to_stability_pool, user_debt);
    assert_eq!(usd_debt_to_redistribute, 0);
    assert_eq!(coll_to_redistribute, CollateralAmounts::default());
    assert_eq!(
        coll_to_stability_pool,
        user_collateral.mul_bps(10_000 - FEES)
    );

    assert_eq!(coll_to_liquidator, user_collateral.mul_bps(LIQUIDATOR_RATE));
    assert_eq!(coll_to_clearer, user_collateral.mul_bps(CLEARER_RATE));
}

#[test]
fn test_liquidation_calcs_between_100_and_110_all_sp_and_redis_split() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 105% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(1.05);

    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(5.0);
    let usd_in_sp = USDH::from(0.6);

    let LiquidationBreakdownAmounts {
        usd_debt_to_redistribute,
        usd_debt_to_stability_pool,
        coll_to_redistribute,
        coll_to_stability_pool,
        coll_to_liquidator,
        coll_to_clearer,
    } = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    )
    .unwrap();

    // remaining coll: 1.05 * 0.995 = 1.04475
    // liquidator fee 1.05 * 0.004 = 0.004200000000000001
    // clearer fee 1.05 * 0.001 = 0.0010500000000000002
    // 0.6 * 1.04475 = 0.62685
    // 0.4 * 1.04475 = 0.41790000000000005

    assert_eq!(usd_debt_to_stability_pool, USDH::from(0.6));
    assert_eq!(usd_debt_to_redistribute, USDH::from(0.4));
    assert_eq!(coll_to_redistribute, sol_collateral(0.4179));
    assert_eq!(coll_to_stability_pool, sol_collateral(0.62685));

    assert_eq!(coll_to_liquidator, sol_collateral(0.0042));
    assert_eq!(coll_to_clearer, sol_collateral(0.00105));
}

#[test]
fn test_liquidation_calcs_between_110_and_150_normal_mode() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 130% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(1.30);

    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(5.0);
    let usd_in_sp = USDH::from(0.6);

    let res = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    );

    assert_eq!(res.err().unwrap(), BorrowError::UserWellCollateralized);
}

#[test]
fn test_liquidation_calcs_between_110_and_150_recovery_mode_all_to_sp() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 130% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(1.3);

    // system is 140%, in recovery mode, (2.0 * 1.5 = 3.0)
    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(2.8);
    let usd_in_sp = USDH::from(2.0);

    let LiquidationBreakdownAmounts {
        usd_debt_to_redistribute,
        usd_debt_to_stability_pool,
        coll_to_redistribute,
        coll_to_stability_pool,
        coll_to_liquidator,
        coll_to_clearer,
    } = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    )
    .unwrap();

    // liquidatable coll: 1.1
    // fee = 1.1 * 0.005 = 0.0055
    // fee = 1.1 * 0.001 = 0.0011
    // fee = 1.1 * 0.004 = 0.0044
    // coll to sp = 1.1 * 0.995 = 1.0945

    // TODO: add integration test, ensure balance stays the same
    // allow user to withdraw
    // allow user to top up, ensure redistribution work
    assert_eq!(usd_debt_to_stability_pool, USDH::from(1.0));
    assert_eq!(usd_debt_to_redistribute, USDH::from(0.0));
    assert_eq!(coll_to_redistribute, sol_collateral(0.0));
    assert_eq!(coll_to_stability_pool, sol_collateral(1.0945));

    assert_eq!(coll_to_liquidator, sol_collateral(0.0044));
    assert_eq!(coll_to_clearer, sol_collateral(0.0011));
}

#[test]
fn test_liquidation_calcs_between_110_and_150_recovery_mode_split_sp_redistrib_sp_cannot_absorb() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 130% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(1.3);

    // system is 140%, in recovery mode, (2.0 * 1.5 = 3.0)
    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(2.8);
    let usd_in_sp = USDH::from(0.7);

    let res = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    );

    assert!(res.is_err());
}

#[test]
fn test_liquidation_calcs_between_110_and_150_recovery_mode_split_sp_redistrib_sp_can_absorb() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 130% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(1.3);

    // system is 140%, in recovery mode, (2.0 * 1.5 = 3.0)
    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(2.8);
    let usd_in_sp = USDH::from(1.7);

    let LiquidationBreakdownAmounts {
        usd_debt_to_redistribute,
        usd_debt_to_stability_pool,
        coll_to_redistribute,
        coll_to_stability_pool,
        coll_to_liquidator,
        coll_to_clearer,
    } = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    )
    .unwrap();

    // liquidatable coll = 1.1
    // 0.995 * 1.1 = 1.0945
    // 0.001 * 1.1 = 0.0011
    // 0.004 * 1.1 = 0.0044
    // 0.0052 + 0.0013 = 0.0065
    // 0.0065 / 1.3

    assert_eq!(usd_debt_to_stability_pool, USDH::from(1.0));
    assert_eq!(usd_debt_to_redistribute, USDH::from(0.0));
    assert_eq!(coll_to_redistribute, sol_collateral(0.0));
    assert_eq!(coll_to_stability_pool, sol_collateral(1.0945));

    assert_eq!(coll_to_liquidator, sol_collateral(0.0044));
    assert_eq!(coll_to_clearer, sol_collateral(0.0011));
}

#[test]
fn test_liquidation_calcs_ensure_loss_never_above_110_percent() {
    // SOL/USD 1.0
    let prices = TokenPrices::new(1.0);

    // 130% coll ratio
    let user_debt = USDH::from(1.0);
    let user_collateral = sol_collateral(1.104);

    // system is 140%, in recovery mode, (2.0 * 1.5 = 3.0)
    let global_debt = USDH::from(2.0);
    let global_collateral = sol_collateral(2.8);
    let usd_in_sp = USDH::from(1.7);

    let LiquidationBreakdownAmounts {
        usd_debt_to_redistribute,
        usd_debt_to_stability_pool,
        coll_to_redistribute,
        coll_to_stability_pool,
        coll_to_liquidator,
        coll_to_clearer,
    } = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        global_debt,
        &global_collateral,
        usd_in_sp,
        &prices,
    )
    .unwrap();

    let remaining_coll = user_collateral
        .sub(&coll_to_redistribute)
        .sub(&coll_to_stability_pool)
        .sub(&coll_to_liquidator)
        .sub(&coll_to_clearer);

    let loss = user_collateral.sub(&remaining_coll);
    assert_eq!(loss, sol_collateral(1.10));
}

#[test]
fn test_position_adjustment_normal_mode_allow_if_tcr_above_ccr_open_position() {
    // allow to open new position if the new TCR > CCR
    // start with TCR at 150%

    use CollateralToken::SOL;
    let (mut market, mut spool, px, now, _) = utils::set_up_above_ccr_market();

    let (mut new_user, new_borrow, new_deposit) = (
        UserMetadata::default(),
        USDH::from(1000.0),
        sol_to_lamports(1000.0),
    );

    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    borrowing_operations::deposit_collateral(&mut market, &mut new_user, new_deposit, SOL).unwrap();
    borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        &px,
        now,
    )
    .unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);
}
#[test]
fn test_position_adjustment_normal_mode_allow_if_tcr_above_ccr_add_coll() {
    use CollateralToken::SOL;
    let (mut market, _spool, px, _now, mut user) = utils::set_up_above_ccr_market();

    let new_deposit = sol_to_lamports(100.0);

    // unwrap shouldn't panic
    borrowing_operations::deposit_collateral(&mut market, &mut user, new_deposit, SOL).unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);
}
#[test]
fn test_position_adjustment_normal_mode_allow_if_tcr_above_ccr_withdraw_coll() {
    use CollateralToken::SOL;
    let (mut market, _spool, px, _now, mut user) = utils::set_up_above_ccr_market();

    let new_deposit = sol_to_lamports(100.0);
    borrowing_operations::deposit_collateral(&mut market, &mut user, new_deposit, SOL).unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);

    // now withdraw
    let new_withdraw = sol_to_lamports(90.0);
    borrowing_operations::withdraw_collateral(&mut market, &mut user, new_withdraw, SOL, &px)
        .unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);
}

#[test]
fn test_position_adjustment_normal_mode_allow_if_tcr_above_ccr_borrow_more() {
    use CollateralToken::SOL;
    let (mut market, mut spool, px, now, mut user) = utils::set_up_above_ccr_market();

    let new_deposit = sol_to_lamports(100.0);
    borrowing_operations::deposit_collateral(&mut market, &mut user, new_deposit, SOL).unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);

    // now borrow more
    let new_borrow = USDH::from(90.0);
    borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut user,
        &mut spool,
        new_borrow,
        &px,
        now,
    )
    .unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);
}
#[test]
fn test_position_adjustment_normal_mode_allow_if_tcr_above_ccr_repay() {
    use CollateralToken::SOL;
    let (mut market, _spool, px, _now, mut user) = utils::set_up_above_ccr_market();

    let new_deposit = sol_to_lamports(100.0);
    borrowing_operations::deposit_collateral(&mut market, &mut user, new_deposit, SOL).unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);

    // now repay
    let repay_amount = USDH::from(90.0);
    borrowing_operations::repay_loan(&mut market, &mut user, repay_amount).unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);
}

#[test]
fn test_position_adjustment_normal_mode_block_if_tcr_below_ccr_open_position() {
    use CollateralToken::SOL;
    let (mut market, mut spool, px, now, _) = utils::set_up_above_ccr_market();

    let (mut new_user, new_borrow, new_deposit) = (
        UserMetadata::default(),
        USDH::from(110.0),
        sol_to_lamports(100.0),
    );

    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    borrowing_operations::deposit_collateral(&mut market, &mut new_user, new_deposit, SOL).unwrap();
    let res = borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        &px,
        now,
    );

    assert_eq!(
        res.err().unwrap(),
        crate::BorrowError::OperationBringsSystemToRecoveryMode
    );
}

#[test]
fn test_position_adjustment_normal_mode_block_if_tcr_below_ccr_withdraw_coll() {
    use CollateralToken::SOL;
    let (mut market, _spool, px, _now, mut user) = utils::set_up_above_ccr_market();

    let new_deposit = sol_to_lamports(100.0);
    borrowing_operations::deposit_collateral(&mut market, &mut user, new_deposit, SOL).unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);

    // now withdraw
    let new_withdraw = sol_to_lamports(110.0);
    let res =
        borrowing_operations::withdraw_collateral(&mut market, &mut user, new_withdraw, SOL, &px);

    assert_eq!(
        res.err().unwrap(),
        crate::BorrowError::OperationBringsSystemToRecoveryMode
    );
}

#[test]
fn test_position_adjustment_normal_mode_block_if_tcr_below_ccr_borrow_more() {
    use CollateralToken::SOL;
    let (mut market, mut spool, px, now, mut user) = utils::set_up_above_ccr_market();

    let new_deposit = sol_to_lamports(100.0);
    borrowing_operations::deposit_collateral(&mut market, &mut user, new_deposit, SOL).unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);

    // now borrow more
    let new_borrow = USDH::from(110.0);
    let res = borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut user,
        &mut spool,
        new_borrow,
        &px,
        now,
    );

    assert_eq!(
        res.err().unwrap(),
        crate::BorrowError::OperationBringsSystemToRecoveryMode
    );
}

#[test]
fn test_position_adjustment_recovery_mode_allow_top_up_coll() {
    use CollateralToken::SOL;
    let (mut market, _spool, _px, _now, mut user) = utils::set_up_above_ccr_market();

    // prev prices at 1.52
    let _new_prices = TokenPrices::new(1.4);

    // top up is allowed
    // prices not even taken as arg
    // but this is just to prove it's allowed
    let new_deposit = sol_to_lamports(100.0);
    borrowing_operations::deposit_collateral(&mut market, &mut user, new_deposit, SOL).unwrap();
}

#[test]
fn test_position_adjustment_recovery_mode_allow_repayment() {
    let (mut market, _spool, _px, _now, mut user) = utils::set_up_above_ccr_market();

    // prev prices at 1.52
    let _new_prices = TokenPrices::new(1.4);

    // repay is allowed
    // prices not even taken as arg
    // but this is just to prove it's allowed
    let repay_amount = USDH::from(10.0);
    borrowing_operations::repay_loan(&mut market, &mut user, repay_amount).unwrap();
}

#[test]
fn test_position_adjustment_recovery_mode_disallow_coll_withdraw() {
    let (mut market, _spool, _px, _now, mut user) = utils::set_up_above_ccr_market();

    // prev prices at 1.52
    let new_prices = TokenPrices::new(1.4);
    let withdraw_amount = USDH::from(10.0);
    let res = borrowing_operations::withdraw_collateral(
        &mut market,
        &mut user,
        withdraw_amount,
        CollateralToken::SOL,
        &new_prices,
    );

    assert_eq!(
        res.err().unwrap(),
        crate::BorrowError::CannotWithdrawInRecoveryMode
    );
}

#[test]
fn test_position_adjustment_recovery_mode_disallow_extra_borrow_even_if_above_mcr() {
    use CollateralToken::SOL;
    let (mut market, mut spool, old_prices, now, _user) = utils::set_up_above_ccr_market();

    let (mut new_user, new_borrow, new_deposit) = (
        UserMetadata::default(),
        USDH::from(1000.0),
        sol_to_lamports(1000.0),
    );

    // This is equivalent to an open (due to inactive collateral)
    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    borrowing_operations::deposit_collateral(&mut market, &mut new_user, new_deposit, SOL).unwrap();
    borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        &old_prices,
        now,
    )
    .unwrap();

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &old_prices,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr > 150);

    // Borrow again, should not be allowed
    // prev prices at 1.52
    let new_prices = TokenPrices::new(1.4);
    let res = borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        &new_prices,
        now,
    );
    assert_eq!(
        res.err().unwrap(),
        crate::BorrowError::OperationLowersSystemTCRInRecoveryMode
    );
}

#[test]
fn test_position_adjustment_recovery_mode_disallow_extra_borrow() {
    let (mut market, mut spool, _px, now, mut user) = utils::set_up_above_ccr_market();

    // prev prices at 1.52
    let new_prices = TokenPrices::new(1.4);

    // repay is allowed
    // prices not even taken as arg
    // but this is just to prove it's allowed
    let borrow_amount = USDH::from(10.0);
    let res = borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut user,
        &mut spool,
        borrow_amount,
        &new_prices,
        now,
    );

    assert_eq!(
        res.err().unwrap(),
        crate::BorrowError::OperationLowersSystemTCRInRecoveryMode
    );
}

#[test]
fn test_position_adjustment_recovery_mode_allow_open_position_above_150() {
    use CollateralToken::SOL;
    let (mut market, mut spool, _old_prices, now, _user) = utils::set_up_above_ccr_market();

    // prev prices at 1.52
    let new_prices = TokenPrices::new(1.4);

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &new_prices,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr < 150);

    // This is equivalent to an open (due to inactive collateral)
    let (mut new_user, new_borrow, new_deposit) = (
        UserMetadata::default(),
        USDH::from(1000.0),
        sol_to_lamports(1140.0),
    );
    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    borrowing_operations::deposit_collateral(&mut market, &mut new_user, new_deposit, SOL).unwrap();
    borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        &new_prices,
        now,
    )
    .unwrap();

    // Still in recovery mode, but increases TCR
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &new_prices,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr < 150);
}

#[test]
fn test_position_adjustment_recovery_mode_disallow_open_position_below_150() {
    use CollateralToken::SOL;
    let (mut market, mut spool, _old_prices, now, _user) = utils::set_up_above_ccr_market();

    // prev prices at 1.52
    let new_prices = TokenPrices::new(1.4);

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &new_prices,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr < 150);

    // This is equivalent to an open (due to inactive collateral)
    let (mut new_user, new_borrow, new_deposit) = (
        UserMetadata::default(),
        USDH::from(100.0),
        sol_to_lamports(100.0),
    );
    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    borrowing_operations::deposit_collateral(&mut market, &mut new_user, new_deposit, SOL).unwrap();
    let res = borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        &new_prices,
        now,
    );

    assert_eq!(res.err().unwrap(), crate::BorrowError::NotEnoughCollateral);
}

// Tests:
// - [ ] situation: one user is undercollateralized,
//      - there is one last user that paid everything off
//      - we should set as inactive and num_users -= 1
//      - also the same during redemptions
// - [x] after surplus liquidation is inactive collateral
// - [x] after surplus full redemption - is inactive collateral
// - [x] if repay in full -> collateral becomes inactive
// - basically every single time there is collateral backing 0 debt -> it must be considered inactive
//      - [x] liquidate
//      - [x] full repay
//      - [x] full redeem (not partial)
//      - [x] as a bot -> filler/clearer/redeemer
// - [ ] assert user never loses more than 110%
// - [x] assert can withdraw inactive collateral during recovery mode

#[test]
fn test_after_full_repayment_coll_surplus_is_inactive() {
    use CollateralToken::SOL;
    // prices are 1.52
    let (mut market, mut spool, px, now, _) = utils::set_up_above_ccr_market();
    let (mut new_user, new_borrow, new_deposit) = (
        UserMetadata::default(),
        USDH::from(1000.0),
        sol_to_lamports(1000.0),
    );

    assert_eq!(market.num_active_users, 1);

    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    borrowing_operations::deposit_collateral(&mut market, &mut new_user, new_deposit, SOL).unwrap();
    assert_eq!(market.num_active_users, 1);

    borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        &px,
        now,
    )
    .unwrap();

    assert_eq!(market.num_active_users, 2);
    assert_eq!(new_user.borrowed_stablecoin, USDH::from(1005.0));
    assert_eq!(new_user.deposited_collateral.sol, new_deposit);

    borrowing_operations::repay_loan(&mut market, &mut new_user, USDH::from(1005.0)).unwrap();

    assert_eq!(market.num_active_users, 1);
    assert_eq!(new_user.borrowed_stablecoin, 0);
    assert_eq!(new_user.deposited_collateral.sol, 0);
    assert_eq!(new_user.inactive_collateral.sol, new_deposit);
}

#[test]
fn test_allow_withdraw_inactive_collateral_during_recovery_mode() {
    use CollateralToken::SOL;
    // prices are 1.52
    let (mut market, _spool, _px, _now, _) = utils::set_up_above_ccr_market();
    let (mut new_user, _new_borrow, new_deposit) = (
        UserMetadata::default(),
        USDH::from(100.0),
        sol_to_lamports(100.0),
    );

    assert_eq!(market.num_active_users, 1);

    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    borrowing_operations::deposit_collateral(&mut market, &mut new_user, new_deposit, SOL).unwrap();
    assert_eq!(market.num_active_users, 1);

    // prev prices at 1.52
    let new_prices = TokenPrices::new(1.4);

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &new_prices,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr < 150);

    // assert no panic basically
    borrowing_operations::withdraw_collateral(
        &mut market,
        &mut new_user,
        new_deposit,
        SOL,
        &new_prices,
    )
    .unwrap();
}

#[ignore]
#[quickcheck_macros::quickcheck]
fn test_liquidation_never_more_than_110_loss(
    user_debt: u64,
    global_debt: u64,
    user_coll: u64,
    global_collateral: u64,
    usdh_in_sp: u64,
) -> bool {
    use CollateralToken::SOL;

    let prices = TokenPrices::new(1.0);
    let user_collateral = CollateralAmounts::of_token(user_coll, SOL);

    if let Ok(LiquidationBreakdownAmounts {
        usd_debt_to_redistribute: _,
        usd_debt_to_stability_pool: _,
        coll_to_redistribute,
        coll_to_stability_pool,
        coll_to_liquidator,
        coll_to_clearer,
    }) = liquidation_calcs::calculate_liquidation_effects(
        user_debt,
        &user_collateral,
        u64::max(user_debt, global_debt),
        &CollateralAmounts::of_token(u64::max(user_coll, global_collateral), SOL),
        usdh_in_sp,
        &prices,
    ) {
        let loss = coll_to_redistribute
            .add(&coll_to_stability_pool)
            .add(&coll_to_liquidator)
            .add(&coll_to_clearer);
        let loss_mv = CollateralInfo::calc_market_value_usdh(&prices, &loss);
        let ratio = (loss_mv as f64) / (user_debt as f64);
        return ratio < 1.1;
    } else {
        return true;
    }
}

#[test]
fn test_liquidation_coll_surplus_is_inactive() {
    use CollateralToken::SOL;
    // prices are 1.52
    let (mut market, mut spool, px, now, _) = utils::set_up_above_ccr_market();
    let (mut new_user, new_borrow, new_deposit) = (
        UserMetadata::default(),
        USDH::from(1010.0),
        sol_to_lamports(1000.0),
    );

    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    borrowing_operations::deposit_collateral(&mut market, &mut new_user, new_deposit, SOL).unwrap();
    borrowing_operations::borrow_stablecoin(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        &px,
        now,
    )
    .unwrap();

    let total_user_debt = new_user.borrowed_stablecoin;

    // prev prices at 1.52
    let new_prices = TokenPrices::new(1.4);

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &new_prices,
    )
    .to_percent()
    .unwrap();
    println!("TCR {}%", tcr);
    assert!(tcr < 150);

    let icr = CollateralInfo::calc_coll_ratio(
        new_user.borrowed_stablecoin,
        &new_user.deposited_collateral,
        &new_prices,
    )
    .to_percent()
    .unwrap();
    println!("ICR {}%", icr);
    assert!(icr < 150);

    let mut stability_pool_state = StabilityPoolState::default();
    let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
    let mut sp_provider = StabilityProviderState::default();
    stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut sp_provider);
    stability_pool_operations::provide_stability(
        &mut stability_pool_state,
        &mut sp_provider,
        &mut epoch_to_scale_to_sum,
        USDH::from(1500.0),
        now,
    )
    .unwrap();

    let liquidations = RefCell::new(LiquidationsQueue::default());
    let liquidator = Pubkey::new_unique();
    let res = borrowing_operations::try_liquidate(
        liquidator,
        &mut market,
        &mut new_user,
        &mut stability_pool_state,
        &mut epoch_to_scale_to_sum,
        &new_prices,
        &mut liquidations.borrow_mut(),
        now,
    );
    println!("Res {:?}", res);
    assert!(res.is_ok());

    // debt: 1010
    // coll: 1400
    // 110% -> 1010 * 1.005 = 1015.0499999999998 * 1.1 = 1116.555
    // 1116.555 / 1.4 = 797.5392857142858

    println!("Borrowed {}", total_user_debt);

    assert_eq!(new_user.borrowed_stablecoin, 0);
    assert_eq!(new_user.deposited_collateral, CollateralAmounts::default());
    assert_eq!(
        new_user.inactive_collateral.sol,
        coll_to_lamports(202.460714286, SOL)
    );

    // allow to withdraw
    borrowing_operations::withdraw_collateral(
        &mut market,
        &mut new_user,
        coll_to_lamports(100.0, SOL),
        SOL,
        &new_prices,
    )
    .unwrap();

    assert_eq!(
        new_user.inactive_collateral.sol,
        coll_to_lamports(202.460714286 - 100.0, SOL)
    );
}

mod utils {
    use solana_sdk::native_token::sol_to_lamports;

    use crate::{
        borrowing_market::borrowing_operations,
        utils::{coretypes::USDH, finance::CollateralInfo},
        BorrowingMarketState, CollateralToken, StakingPoolState, TokenPrices, UserMetadata,
    };

    pub fn set_up_above_ccr_market() -> (
        BorrowingMarketState,
        StakingPoolState,
        TokenPrices,
        u64,
        UserMetadata,
    ) {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let mut spool = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now = 0;
        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let px = TokenPrices::new(1.52);
        use CollateralToken::SOL;

        // Start with 150%
        let (borrow, deposit) = (USDH::from(1000.0), sol_to_lamports(1000.0));
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(&mut market, &mut user, deposit, SOL).unwrap();
        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut spool,
            borrow,
            &px,
            now,
        )
        .unwrap();

        let tcr = CollateralInfo::calc_coll_ratio(
            market.stablecoin_borrowed,
            &market.deposited_collateral,
            &px,
        )
        .to_percent()
        .unwrap();
        println!("TCR {}%", tcr);
        assert!(tcr > 150);

        (market, spool, px, now, user)
    }
}
