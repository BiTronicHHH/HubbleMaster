#![allow(unaligned_references)]

use crate::{
    borrowing_market::{borrowing_operations, types::DepositAndBorrowEffects},
    state::CollateralToken,
    utils::{coretypes::USDH, finance::CollateralInfo},
    BorrowError, BorrowingMarketState, StakingPoolState, TokenPrices, UserMetadata,
};
use solana_sdk::native_token::sol_to_lamports;
// Tests
// - [x] Open a single position as an adjustment (normal mode)
// - [x] Open a single position as an adjustment (recovery mode)
// - [x] Adjusting coll up is the same as deposit only
// - [x] Adjusting upwards (from a CR pov) in recovery mode is allowed
// - [x] Adjusting downwards (from a CR pov) in recovery mode is not allowed
// - [ ] adjustment with 0 borrow == simple deposit
// - [ ] adjustment with 0 deposit == simple borrow
// - [ ] adjustment with 0 both == error

#[test]
fn test_deposit_and_borrow_open_single_position_normal_mode() {
    // This test checks that using the deposit_and_borrow endpoint
    // adds active collateral and borrowed amount correctly
    // to an empty user state and an empty market state.

    let (mut market, mut spool, px, now) = utils::setup(2.0);

    let (mut new_user, new_borrow, new_deposit, asset) = (
        UserMetadata::default(),
        USDH::from(1000.0),
        sol_to_lamports(1000.0),
        CollateralToken::SOL,
    );

    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    let DepositAndBorrowEffects {
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault,
        collateral_to_transfer_from_user,
    } = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        new_deposit,
        asset,
        &px,
        now,
    )
    .unwrap();

    let fee = USDH::from(5.0);
    let treasury_fee = USDH::from(5.0 * 0.15);

    assert_eq!(market.deposited_collateral.sol, new_deposit);
    assert_eq!(market.stablecoin_borrowed, new_borrow + fee);
    assert_eq!(new_user.deposited_collateral.sol, new_deposit);
    assert_eq!(new_user.borrowed_stablecoin, new_borrow + fee);

    assert_eq!(amount_mint_to_user, new_borrow);
    assert_eq!(amount_mint_to_fees_vault, fee - treasury_fee);
    assert_eq!(amount_mint_to_treasury_vault, treasury_fee);

    assert_eq!(collateral_to_transfer_from_user.sol, new_deposit);
}

#[test]
fn test_deposit_and_borrow_open_first_position_into_recovery_mode_disallowed() {
    let (mut market, mut spool, px, now) = utils::setup(1.4);

    let (mut new_user, new_borrow, new_deposit, asset) = (
        UserMetadata::default(),
        USDH::from(1000.0),
        sol_to_lamports(1000.0),
        CollateralToken::SOL,
    );

    borrowing_operations::approve_trove(&mut market, &mut new_user).unwrap();
    let res = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut new_user,
        &mut spool,
        new_borrow,
        new_deposit,
        asset,
        &px,
        now,
    );

    assert_eq!(
        res.err().unwrap(),
        BorrowError::OperationBringsSystemToRecoveryMode
    );
}

#[test]
fn test_deposit_and_borrow_adjust_existing_into_recovery_mode_disallowed() {
    let (borrow, deposit) = (USDH::from(1000.0), sol_to_lamports(1000.0));
    let (mut market, mut spool, px, now, mut user) = utils::setup_with_user(1.6, borrow, deposit);

    // Assert it's well collateralized
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(tcr > 150);

    // borrow 100
    // deposit 100 * 1.6 = 160 coll
    // 160 coll ratio

    // + 140 more to borrow = 200 borrow
    // + 40 coll -> 140 * 1.6 = 224
    // 224 / 200 = 1.12 coll ratio
    // brings system to Recovery, disallow

    // Deposit more and borrow
    let (borrow, deposit) = (USDH::from(140.0), sol_to_lamports(40.0));
    let res = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut user,
        &mut spool,
        borrow,
        deposit,
        CollateralToken::SOL,
        &px,
        now,
    );

    assert_eq!(
        res.err().unwrap(),
        BorrowError::OperationBringsSystemToRecoveryMode
    );
}

#[test]
fn test_deposit_and_borrow_improve_personal_cr_even_if_still_in_recovery() {
    // can (deposit+borrow) to improve personal CR even if still Recovery

    let (borrow, deposit) = (USDH::from(10000.0), sol_to_lamports(10000.0));
    let (mut market, mut spool, _px, now, _user) = utils::setup_with_user(1.6, borrow, deposit);

    let (borrow, deposit) = (USDH::from(10000.0), sol_to_lamports(10000.0));
    let mut second_user = utils::new_user(&mut market, &mut spool, 1.6, borrow, deposit);

    // Assert it's in recovery mode with new price
    let px = TokenPrices::new(1.4);
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(tcr < 150);

    let second_user_mcr = CollateralInfo::calc_coll_ratio(
        second_user.borrowed_stablecoin,
        &second_user.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(second_user_mcr < 150);
    println!("second_user_mcr {}", second_user_mcr);

    // Deposit and borrow out of recovery mode at 0 fees
    // Can only borrow if new ICR > 150%
    let (borrow_1, deposit_1) = (USDH::from(1000.0), sol_to_lamports(2000.0));
    let DepositAndBorrowEffects {
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault: _,
        collateral_to_transfer_from_user,
    } = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut second_user,
        &mut spool,
        borrow_1,
        deposit_1,
        CollateralToken::SOL,
        &px,
        now,
    )
    .unwrap();

    let second_user_mcr = CollateralInfo::calc_coll_ratio(
        second_user.borrowed_stablecoin,
        &second_user.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    println!("second_user_mcr {}", second_user_mcr);

    // still in recovery
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(tcr < 150);

    // effects
    assert_eq!(amount_mint_to_user, borrow_1);
    assert_eq!(amount_mint_to_fees_vault, 0);
    assert_eq!(collateral_to_transfer_from_user.sol, deposit_1);

    // balances
    let fee = USDH::from(50.0); // only once
    assert_eq!(second_user.deposited_collateral.sol, deposit + deposit_1);
    assert_eq!(second_user.borrowed_stablecoin, (borrow + fee) + borrow_1);
}

#[test]
fn test_deposit_and_borrow_adjust_existing_out_of_recovery_mode_allowed() {
    let (borrow, deposit) = (USDH::from(1000.0), sol_to_lamports(1000.0));
    let (mut market, mut spool, _px, now, mut user) = utils::setup_with_user(1.6, borrow, deposit);

    // Assert it's in recovery mode with new price
    let px = TokenPrices::new(1.4);
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(tcr < 150);

    // Deposit and borrow out of recovery mode at 0 fees
    let (borrow_1, deposit_1) = (USDH::from(1000.0), sol_to_lamports(2000.0));
    let DepositAndBorrowEffects {
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault: _,
        collateral_to_transfer_from_user,
    } = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut user,
        &mut spool,
        borrow_1,
        deposit_1,
        CollateralToken::SOL,
        &px,
        now,
    )
    .unwrap();

    // effects
    assert_eq!(amount_mint_to_user, borrow_1);
    assert_eq!(amount_mint_to_fees_vault, 0);
    assert_eq!(collateral_to_transfer_from_user.sol, deposit_1);

    // balances
    let fee = USDH::from(5.0);
    assert_eq!(market.deposited_collateral.sol, deposit + deposit_1);
    assert_eq!(market.stablecoin_borrowed, (borrow + fee) + borrow_1);
    assert_eq!(user.deposited_collateral.sol, deposit + deposit_1);
    assert_eq!(user.borrowed_stablecoin, (borrow + fee) + borrow_1);
}

#[test]
fn test_deposit_and_borrow_adjust_existing_in_normal_mode() {
    let (borrow, deposit) = (USDH::from(1000.0), sol_to_lamports(1000.0));
    let (mut market, mut spool, px, now, mut user) = utils::setup_with_user(1.6, borrow, deposit);

    // Deposit more and borrow
    let DepositAndBorrowEffects {
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault,
        collateral_to_transfer_from_user,
    } = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut user,
        &mut spool,
        borrow,
        deposit,
        CollateralToken::SOL,
        &px,
        now,
    )
    .unwrap();

    let fee = USDH::from(5.0);
    let treasury_fee = USDH::from(5.0 * 0.15);

    assert_eq!(market.deposited_collateral.sol, deposit * 2);
    assert_eq!(market.stablecoin_borrowed, (borrow + fee) * 2);
    assert_eq!(user.deposited_collateral.sol, deposit * 2);
    assert_eq!(user.borrowed_stablecoin, (borrow + fee) * 2);

    assert_eq!(amount_mint_to_user, borrow);
    assert_eq!(amount_mint_to_fees_vault, fee - treasury_fee);
    assert_eq!(amount_mint_to_treasury_vault, treasury_fee);
    assert_eq!(collateral_to_transfer_from_user.sol, deposit);
}

#[test]
fn test_deposit_and_borrow_open_single_position_recovery_mode_disallowed_below_150() {
    let (mut market, mut spool, px, now, _first_user) =
        utils::setup_with_user(1.6, USDH::from(1000.0), sol_to_lamports(1000.0));

    // Assert it's well collateralized
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(tcr > 150);

    // Change prices, assert in recovery mode
    let px = TokenPrices::new(1.4);
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(tcr < 150);

    // Try to open a position this way, must be disallowed
    let (mut second_user, second_borrow, second_deposit, asset) = (
        UserMetadata::default(),
        USDH::from(100.0),
        sol_to_lamports(100.0),
        CollateralToken::SOL,
    );
    let res = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut second_user,
        &mut spool,
        second_borrow,
        second_deposit,
        asset,
        &px,
        now,
    );

    // Collateral should be enough to cover above 150%
    assert_eq!(res.err().unwrap(), BorrowError::NotEnoughCollateral);
}

#[test]
fn test_deposit_and_borrow_open_single_position_recovery_mode_allowed_above_150() {
    let (first_borrow, first_deposit) = (USDH::from(1000.0), sol_to_lamports(1000.0));
    let (mut market, mut spool, px, now, first_user) =
        utils::setup_with_user(1.6, first_borrow, first_deposit);

    // Assert it's well collateralized
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(tcr > 150);

    // Change prices, assert in recovery mode
    let px = TokenPrices::new(1.4);
    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        &px,
    )
    .to_percent()
    .unwrap();
    assert!(tcr < 150);

    // Try to open a position above 150%, must be allowed
    let (mut second_user, second_borrow, second_deposit, asset) = (
        UserMetadata::default(),
        USDH::from(1000.0),
        sol_to_lamports(2000.0),
        CollateralToken::SOL,
    );
    let DepositAndBorrowEffects {
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault,
        collateral_to_transfer_from_user,
    } = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut second_user,
        &mut spool,
        second_borrow,
        second_deposit,
        asset,
        &px,
        now,
    )
    .unwrap();

    let first_fee = USDH::from(5.0);
    let second_fee = USDH::from(0.0); // because borrowing in recovery mode

    assert_eq!(
        market.deposited_collateral.sol,
        first_deposit + second_deposit
    );
    assert_eq!(first_user.deposited_collateral.sol, first_deposit);
    assert_eq!(first_user.borrowed_stablecoin, first_borrow + first_fee);
    assert_eq!(second_user.deposited_collateral.sol, second_deposit);
    assert_eq!(second_user.borrowed_stablecoin, second_borrow + second_fee);

    assert_eq!(
        market.stablecoin_borrowed,
        first_borrow + second_borrow + first_fee + second_fee
    );

    assert_eq!(amount_mint_to_user, second_borrow);
    assert_eq!(amount_mint_to_fees_vault, second_fee);
    assert_eq!(amount_mint_to_treasury_vault, 0);

    assert_eq!(collateral_to_transfer_from_user.sol, second_deposit)
}

#[test]
fn test_deposit_and_borrow_adjusts_correctly_inactive_position() {
    // This test checks that using the deposit_and_borrow endpoint
    // adds active collateral and borrowed amount correctly
    // to existing (inactive) user state and inactive market state.

    let (mut market, mut spool, px, now) = utils::setup(1.6);
    let (mut user, borrow, deposit, asset) = (
        UserMetadata::default(),
        USDH::from(1000.0),
        sol_to_lamports(1000.0),
        CollateralToken::SOL,
    );

    // Deposit some
    borrowing_operations::deposit_collateral(&mut market, &mut user, deposit, asset).unwrap();
    // Deposit more and borrow
    let DepositAndBorrowEffects {
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault,
        collateral_to_transfer_from_user,
    } = borrowing_operations::deposit_and_borrow(
        &mut market,
        &mut user,
        &mut spool,
        borrow,
        deposit,
        asset,
        &px,
        now,
    )
    .unwrap();

    let fee = USDH::from(5.0);
    let treasury_fee = USDH::from(5.0 * 0.15);

    assert_eq!(market.deposited_collateral.sol, deposit * 2);
    assert_eq!(market.stablecoin_borrowed, borrow + fee);
    assert_eq!(user.deposited_collateral.sol, deposit * 2);
    assert_eq!(user.borrowed_stablecoin, borrow + fee);

    assert_eq!(amount_mint_to_user, borrow);
    assert_eq!(amount_mint_to_fees_vault, fee - treasury_fee);
    assert_eq!(amount_mint_to_treasury_vault, treasury_fee);

    assert_eq!(collateral_to_transfer_from_user.sol, deposit);
}

mod utils {
    use crate::borrowing_market::borrowing_operations;

    use super::*;
    pub fn setup(prices: f64) -> (BorrowingMarketState, StakingPoolState, TokenPrices, u64) {
        let mut market = BorrowingMarketState::new();
        let spool = StakingPoolState {
            treasury_fee_rate: 1_500, // bps: 15%
            ..Default::default()
        };
        let now = 0;
        let px = TokenPrices::new(prices);

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        (market, spool, px, now)
    }

    pub fn setup_with_user(
        price: f64,
        borrow: u64,
        deposit: u64,
    ) -> (
        BorrowingMarketState,
        StakingPoolState,
        TokenPrices,
        u64,
        UserMetadata,
    ) {
        let (mut market, mut spool, px, now) = utils::setup(price);
        let (mut first_user, asset) = (UserMetadata::default(), CollateralToken::SOL);

        // First user, deposit and borrow in normal mode, result is overcollateralized
        borrowing_operations::approve_trove(&mut market, &mut first_user).unwrap();
        borrowing_operations::deposit_collateral(&mut market, &mut first_user, deposit, asset)
            .unwrap();
        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut first_user,
            &mut spool,
            borrow,
            &px,
            now,
        )
        .unwrap();
        (market, spool, px, now, first_user)
    }

    pub fn new_user(
        market: &mut BorrowingMarketState,
        spool: &mut StakingPoolState,
        px: f64,
        borrow: u64,
        deposit: u64,
    ) -> UserMetadata {
        let mut user = UserMetadata::default();
        borrowing_operations::approve_trove(market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(market, &mut user, deposit, CollateralToken::SOL)
            .unwrap();
        borrowing_operations::borrow_stablecoin(
            market,
            &mut user,
            spool,
            borrow,
            &TokenPrices::new(px),
            0,
        )
        .unwrap();

        user
    }
}
