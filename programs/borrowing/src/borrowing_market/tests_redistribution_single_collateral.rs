#![allow(unaligned_references)]
#[cfg(test)]
mod tests {

    const SE: u64 = 10;

    use std::cell::RefCell;

    use anchor_lang::prelude::Pubkey;

    use crate::borrowing_market::tests_utils;
    use crate::state::StabilityToken;
    use crate::{
        assert_fuzzy_eq,
        borrowing_market::{borrowing_operations, borrowing_rate::BorrowSplit, tests_utils::utils},
        deposited,
        stability_pool::{
            liquidations_queue,
            stability_pool_operations::{self},
            tests_utils::utils::new_stability_users,
        },
        staking_pool::staking_pool_operations,
        state::epoch_to_scale_to_sum::EpochToScaleToSum,
        utils::{
            consts::{CLEARER_RATE, LIQUIDATOR_RATE, ONE},
            coretypes::{SOL, USDH},
        },
        BorrowError, BorrowingMarketState, CollateralAmounts, CollateralToken, LiquidationsQueue,
        StabilityPoolState, StakingPoolState, TokenPrices, UserMetadata,
    };

    #[test]
    fn test_borrowing_liquidate_redistribution_single() {
        let (
            mut market,
            mut stability_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            mut staking_pool_state,
            now_timestamp,
        ) = tests_utils::utils::set_up_market();

        // 201.0 * 1.1 = 221.1 /
        let sol_deposits = SOL::from(2.0);
        let liquidation_price = 110.5;

        let mut total_amount_borrowed = 0;
        let mut total_amount_deposited = 0;
        let amount_to_borrow = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let total_borrowed = borrow_split.amount_to_borrow;

        let count = 10;
        let mut users: Vec<UserMetadata> = (0..count)
            .map(|i| {
                let mut user = UserMetadata::default();
                borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

                // The first user gets little, the rest get double
                // such that system CR is > 150% and it's not in recovery mode
                let deposit_amount = if i == 0 {
                    sol_deposits
                } else {
                    sol_deposits * 2
                };
                borrowing_operations::deposit_collateral(
                    &mut market,
                    &mut user,
                    deposit_amount,
                    CollateralToken::SOL,
                )
                .unwrap();
                total_amount_deposited += deposit_amount;

                borrowing_operations::borrow_stablecoin(
                    &mut market,
                    &mut user,
                    &mut staking_pool_state,
                    amount_to_borrow,
                    &TokenPrices::new(liquidation_price + 100.0),
                    now_timestamp,
                )
                .unwrap();

                total_amount_borrowed += total_borrowed;

                user
            })
            .collect();

        println!("User Before {:#?}", users[0]);
        println!("Market Before {:#?}", market);

        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(liquidation_price),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        // There are 10 users with
        // - 1.000.000 stablecoin borrowed each
        // - 10000000000 sol deposited each

        // After 1st user gets liquidated (MINUS COLL FEE 0.005) ->
        // 1.000.000 is split in 9 = 1000000  / 9 = 111111.11111111111 (RPT)
        // 9950000000 is split in 9 = 9950000000 / 9 = 1105555555.5555556 (RPT)
        // minus liquidation fee
        // expected coll each: 10000000000 + 1105555555 = 11105555555
        // expected debt each: 1000000 + 111111 = 1111111

        borrowing_operations::refresh_positions(&mut market, &mut users[0]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut users[1]).unwrap();

        println!("User After {:#?}", users[0]);
        println!("Market After {:#?}", market);

        let liquidator_fee = (0.005 * (sol_deposits as f64)) as u64;

        // Liquidated user
        assert_eq!(users[0].borrowed_stablecoin, 0);
        assert_eq!(deposited!(users[0], CollateralToken::SOL), 0);

        // Redistributed user
        assert_eq!(
            users[1].borrowed_stablecoin,
            borrow_split.amount_to_borrow + borrow_split.amount_to_borrow / ((count - 1) as u64)
        );
        assert_eq!(
            users[1].deposited_collateral.sol,
            (total_amount_deposited - liquidator_fee) / (count - 1)
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_amount_deposited - liquidator_fee
        );
        assert_eq!(market.stablecoin_borrowed, total_amount_borrowed);
    }

    #[test]
    fn test_edistribution_deposit_after_redistribution_noop() {
        // Two users, one liquidate & redistribute
        // Third user deposits
        // Check balances, noop

        let (
            mut market,
            mut stability_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            mut staking_pool_state,
            now_timestamp,
        ) = tests_utils::utils::set_up_market();

        let amount_to_borrow = USDH::from(200.0);
        let bsplit = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let deposit = SOL::from(15.0);
        let mut first_two_users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            2,
            amount_to_borrow,
            deposit,
            now_timestamp,
        );

        borrowing_operations::try_liquidate(
            Pubkey::new_unique(),
            &mut market,
            &mut first_two_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        let third_user = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            1,
            amount_to_borrow * 10,
            deposit * 10,
            now_timestamp,
        );

        let mut users = vec![];
        users.extend(first_two_users);
        users.extend(third_user);

        for user in users.iter_mut() {
            borrowing_operations::refresh_positions(&mut market, user).unwrap();
        }

        let liq_fee = deposit * 50 / 10_000;
        assert_eq!(users[0].borrowed_stablecoin, 0);
        assert_eq!(users[1].borrowed_stablecoin, bsplit.amount_to_borrow * 2);
        assert_eq!(users[2].borrowed_stablecoin, bsplit.amount_to_borrow * 10);

        assert_eq!(users[0].deposited_collateral.sol, 0);
        assert_fuzzy_eq!(users[1].deposited_collateral.sol, deposit * 2 - liq_fee, 2);
        assert_eq!(users[2].deposited_collateral.sol, deposit * 10);
    }

    #[test]
    fn test_borrowing_two_users_fully_redistribute() {
        let (
            mut market,
            mut stability_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            mut staking_pool_state,
            now_timestamp,
        ) = tests_utils::utils::set_up_market();

        let amount_to_borrow = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let deposit_collateral = SOL::from(15.0);

        let mut users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            2,
            amount_to_borrow,
            deposit_collateral,
            now_timestamp,
        );

        for (_i, _user) in users.iter().enumerate() {
            assert_eq!(_user.deposited_collateral.sol as u64, deposit_collateral);
            assert_eq!(_user.borrowed_stablecoin, borrow_split.amount_to_borrow);
        }

        let total_deposited_amount = deposit_collateral * 2;
        let total_borrowed_amount = borrow_split.amount_to_borrow * 2;
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        // Assert first user has nothing
        // Assert second user has all

        for (_i, user) in users.iter_mut().enumerate() {
            borrowing_operations::refresh_positions(&mut market, user).unwrap();
        }

        let liquidator_fee = (0.005 * (deposit_collateral as f64)) as u64;

        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount - liquidator_fee
        );

        // Liquidated user
        assert_eq!(users[0].borrowed_stablecoin, 0);
        assert_eq!(deposited!(users[0], CollateralToken::SOL), 0);

        // Redistributed user
        assert_fuzzy_eq!(users[1].borrowed_stablecoin, total_borrowed_amount, 2);
        assert_fuzzy_eq!(
            users[1].deposited_collateral.sol as u64,
            total_deposited_amount - liquidator_fee,
            3
        );

        // Market
        assert_eq!(
            market.deposited_collateral.sol,
            total_deposited_amount - liquidator_fee
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
    }

    #[test]
    fn test_borrowing_three_users_fully_redistribute_based_on_usd_amounts() {
        // Upon redistribution debt and collateral is reallocated based on the user's usd deposits
        // If a user has 75% of the whole pool of usd, then they get 75% of the redistributed col and debt
        // the other users take the remaining 25% of the redistributed usd and coll
        // In this test we have one user having 1/3 of the pool and the other 2/3 of the pool

        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let amount_to_borrow = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let deposit_collateral = SOL::from(20.0);

        // the user to be liquidated
        let borrower = &mut utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            1,
            amount_to_borrow,
            deposit_collateral,
            now_timestamp,
        )[0];

        // the users to gain liquidations
        let borrow_splits = [
            BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps),
            BorrowSplit::from_amount(amount_to_borrow * 2, market.base_rate_bps),
        ];

        let mut users = utils::new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            2,
            &[amount_to_borrow, amount_to_borrow * 2],
            &[
                CollateralAmounts::of_token(deposit_collateral.into(), CollateralToken::SOL),
                CollateralAmounts::of_token(deposit_collateral.into(), CollateralToken::SOL),
            ],
            now_timestamp,
        );

        // the user to be liquidated
        assert_eq!(
            deposited!(users[0], CollateralToken::SOL),
            deposit_collateral
        );
        assert_eq!(borrower.borrowed_stablecoin, borrow_split.amount_to_borrow);

        // the users to gain liquidations
        for (i, _user) in users.iter().enumerate() {
            assert_eq!(_user.deposited_collateral.sol as u64, deposit_collateral);
            assert_eq!(_user.borrowed_stablecoin, borrow_splits[i].amount_to_borrow);
        }

        let total_deposited_amount = deposit_collateral * 3;
        let total_borrowed_amount = borrow_split.amount_to_borrow * 4;
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            borrower,
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        // // Assert first user has nothing
        // // Assert second user has all

        for (_i, user) in users.iter_mut().enumerate() {
            borrowing_operations::refresh_positions(&mut market, user).unwrap();
        }

        let liquidator_fee = (0.005 * (deposit_collateral as f64)) as u64;

        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount - liquidator_fee
        );

        // Liquidated user
        assert_eq!(borrower.borrowed_stablecoin, 0);
        assert_eq!(deposited!(borrower, CollateralToken::SOL), 0);

        let redistributed_debt = borrow_split.amount_to_borrow;
        let redistributed_coll = deposit_collateral - liquidator_fee;
        // First user gets 1/3 of the debt and 1/3 of the collateral
        assert_fuzzy_eq!(
            users[0].borrowed_stablecoin,
            borrow_split.amount_to_borrow + redistributed_debt * 1 / 3,
            2
        );
        assert_eq!(
            users[0].deposited_collateral.sol,
            deposit_collateral + redistributed_coll / 3
        );

        // Second user gets 1/3 of the debt and 1/3 of the collateral
        assert_fuzzy_eq!(
            users[1].borrowed_stablecoin,
            borrow_split.amount_to_borrow * 2 + redistributed_debt * 2 / 3,
            2
        );
        assert_eq!(
            users[1].deposited_collateral.sol,
            deposit_collateral + redistributed_coll * 2 / 3
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount - liquidator_fee
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
    }

    #[test]
    fn test_borrowing_one_user_cannot_liquidate() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let amount_to_borrow = USDH::from(200.0);
        let _borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);

        let deposit_collateral = SOL::from(30.0);

        let mut users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            1,
            amount_to_borrow,
            deposit_collateral,
            now_timestamp,
        );
        let liquidator = Pubkey::new_unique();
        let res = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_borrowing_liquidation_split_between_stability_pool_and_redistribution() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;
        let liquidations = RefCell::new(LiquidationsQueue::default());

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        // 100.5 * 1.1 = 110.55000000000001
        // 110.55000000000001 / 15.0 = 7.370000000000001

        let amount_to_borrow = USDH::from(1000.0);
        let deposit_collateral = SOL::from(150.0);
        let liquidation_prices = 7.36;
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);

        let num_borrowers = 10;
        let mut borrowing_users = utils::new_borrowing_users_with_price(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            amount_to_borrow,
            deposit_collateral,
            liquidation_prices + 100.0,
            now_timestamp,
        );

        // The stability pool should not be able to cover the debt of the
        // liquidated user, We liquidate 100 USD, we cover 40 usd (2 users) from
        // the stability pool and the rest gets redistributed
        let num_sp_users = 2;
        let mut stability_providers = new_stability_users(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            num_sp_users,
            200.0,
        );

        let sp_usd_deposits: u64 = stability_providers
            .iter()
            .map(|s| s.deposited_stablecoin)
            .sum();

        println!("User before liquidation: {:?}", borrowing_users[0]);
        println!("Borrowing market before liq {}", market.to_state_string());
        println!("SP before liq {}", stability_pool_state.to_state_string());
        let liquidator = Pubkey::new_unique();
        let liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(liquidation_prices),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        let liquidator_fee = deposit_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let clearer_fee = deposit_collateral * (CLEARER_RATE as u64) / 10_000;
        assert_eq!(
            liquidator_fee,
            liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol as u64
        );

        println!("After liq {}", market.to_state_string());
        println!("After liq {}", stability_pool_state.to_state_string());

        // Before liquidation
        // User debt: 10050000
        // User collateral: 15000000000
        // Stability pool: 400000

        // Liquidator fee: 0.005 * 15000000000 = 75000000

        // Remaining collateral = 15000000000 - 75000000 = 14925000000
        // SP takes 400000 / 10050000 = 0.03980099502487562 of the collateral and debt
        // SP takes 0.03980099502487562 * 10050000 = 400000 debt
        // SP takes 0.03980099502487562 * 14925000000 = 594029850.7462686
        // also there is a new epoch

        // Stability pool users have 0 balance, but a pending gain of
        // 594029850.7462686 / 2 = 297014925.3731343 = 2985000000 each

        // Borrowing market users get redistributed 0.6 of the debt & coll
        // 0.6 * 1000000 = 600000
        // 0.6 * 14925000000 = 8955000000
        // each user gets an extra
        // 600000 / 9  = 66666.66666666667 debt
        // 8955000000 / 9  = 995000000 collateral

        // num active users decreases

        // clear all gains first before harvesting
        {
            use CollateralToken::*;
            let clearing_agent = Pubkey::new_unique();
            for token in [SOL, ETH, BTC, FTT, RAY, SRM] {
                liquidations_queue::clear_liquidation_gains(
                    &mut liquidations.borrow_mut(),
                    token,
                    clearing_agent,
                    now_timestamp,
                );
            }
        }

        println!(
            "Stability Provider {}",
            stability_providers[0].to_state_string()
        );

        // 1. Check stability pool
        for token in [
            StabilityToken::SOL,
            StabilityToken::ETH,
            StabilityToken::BTC,
            StabilityToken::FTT,
            StabilityToken::RAY,
            StabilityToken::SRM,
        ] {
            stability_pool_operations::harvest_liquidation_gains(
                &mut stability_pool_state,
                &mut stability_providers[0],
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                0,
                token,
            )
            .unwrap();
        }
        let stability_pool_coll_absorbed = 59402985074;

        {
            println!(
                "Stability Provider {}",
                stability_providers[0].to_state_string()
            );

            let user_gains_pending = &stability_providers[0].pending_gains_per_user;
            let user_gains_cumulative = &stability_providers[0].cumulative_gains_per_user;
            let user_deposits = &stability_providers[0].deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;

            assert_eq!(user_gains_pending.sol as u64, 0);
            assert_eq!(user_gains_cumulative.sol as u64, 29701492537);
            assert_eq!(
                total_gains_cumulative.sol as u64,
                stability_pool_coll_absorbed
            );

            assert_fuzzy_eq!((*total_user_deposits as u64), 0, SE);
            assert_fuzzy_eq!((*user_deposits as u64), 0, SE);

            // Product gets reset and epoch incremented
            assert_eq!(stability_pool_state.p, ONE);
            assert_eq!(stability_pool_state.current_epoch, 1);
        }

        // 2. Check borrowing market
        // let sp_ratio = 400000.0 / 1005000.0;
        let stability_pool_debt_absored = sp_usd_deposits;

        let total_deposited_amount = deposit_collateral * (num_borrowers as u64);
        let total_borrowed_amount = borrow_split.amount_to_borrow * num_borrowers as u64;

        for (_i, _user) in borrowing_users.iter_mut().enumerate() {
            borrowing_operations::refresh_positions(&mut market, _user).unwrap();
        }

        // Liquidated user
        assert_eq!(borrowing_users[0].borrowed_stablecoin, 0);
        assert_eq!(deposited!(borrowing_users[0], CollateralToken::SOL), 0);

        // Redistributed user
        let debt_per_user = borrow_split.amount_to_borrow;
        println!(
            "stability_pool_debt_absored {} debt_per_user {}",
            stability_pool_debt_absored, debt_per_user
        );
        assert_fuzzy_eq!(
            borrowing_users[1].borrowed_stablecoin,
            debt_per_user
                + ((debt_per_user - stability_pool_debt_absored) / (num_borrowers - 1) as u64),
            2
        );
        let col_redistributed =
            (deposit_collateral - stability_pool_coll_absorbed) / ((num_borrowers - 1) as u64);

        // TODO: fix fuzzy
        assert_fuzzy_eq!(
            borrowing_users[1].deposited_collateral.sol as u64,
            deposit_collateral + col_redistributed,
            SOL::from(0.1)
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount - stability_pool_coll_absorbed - liquidator_fee - clearer_fee
        );
        assert_eq!(
            market.stablecoin_borrowed,
            total_borrowed_amount - stability_pool_debt_absored
        );

        println!("Redistrib User after liquidation: {:?}", borrowing_users[1]);
    }

    #[test]
    fn test_borrowing_deposit_after_redistribution() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;
        let liquidations = RefCell::new(LiquidationsQueue::default());

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let borrow_per_user = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let debt_per_user = borrow_split.amount_to_borrow;

        let deposit_collateral = SOL::from(15.0);

        let num_borrowers = 10;
        let mut borrowing_users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            borrow_per_user,
            deposit_collateral,
            now_timestamp,
        );
        let liquidator = Pubkey::new_unique();
        let liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        let liquidator_fee = deposit_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let clearer_fee = deposit_collateral * (CLEARER_RATE as u64) / 10_000;
        assert_eq!(
            liquidator_fee,
            liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol as u64
        );

        assert_eq!(
            clearer_fee,
            liquidation_effects
                .liquidation_event
                .collateral_gain_to_clearer
                .sol as u64
        );

        // There are 10 users with
        // - 1.000.000 stablecoin borrowed each
        // - 10000000000 sol deposited each

        // After 1st user gets liquidated (MINUS COLL FEE 0.005) ->
        // 1.000.000 is split in 9 = 1000000  / 9 = 111111.11111111111 (RPT)
        // 9950000000 is split in 9 = 9950000000 / 9 = 1105555555.5555556 (RPT)
        // minus liquidation fee
        // expected coll each: 10000000000 + 1105555555 = 11105555555
        // expected debt each: 1000000 + 111111 = 1111111

        println!("borrowed {}", borrowing_users[1].borrowed_stablecoin);

        let extra_stablecoin_borrow_to_borrow = USDH::from(5.0);
        let extra_stablecoin_borrow =
            BorrowSplit::from_amount(extra_stablecoin_borrow_to_borrow, market.base_rate_bps);
        let extra_collateral_deposit = SOL::from(3.3);

        borrowing_operations::deposit_collateral(
            &mut market,
            &mut borrowing_users[1],
            extra_collateral_deposit,
            CollateralToken::SOL,
        )
        .unwrap();

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut borrowing_users[1],
            &mut staking_pool_state,
            extra_stablecoin_borrow.amount_to_borrow - extra_stablecoin_borrow.fees_to_pay,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        // Redistributed user
        assert_eq!(
            borrowing_users[1].borrowed_stablecoin,
            debt_per_user
                + extra_stablecoin_borrow.amount_to_borrow
                + (debt_per_user / (num_borrowers - 1) as u64)
        );
        assert_eq!(
            borrowing_users[1].deposited_collateral.sol as u64,
            deposit_collateral
                + extra_collateral_deposit
                + ((deposit_collateral - liquidator_fee - clearer_fee)
                    / ((num_borrowers - 1) as u64))
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            (deposit_collateral * (num_borrowers as u64) - liquidator_fee - clearer_fee
                + extra_collateral_deposit)
        );
        assert_eq!(
            market.stablecoin_borrowed,
            debt_per_user * (num_borrowers as u64) + extra_stablecoin_borrow.amount_to_borrow
        );
    }

    #[test]
    fn test_borrowing_withdraw_more_collateral_than_deposited_after_liquidation_and_no_redistribution(
    ) {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let amount_to_borrow = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);

        let deposit_collateral = SOL::from(15.0);

        let mut users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            2,
            amount_to_borrow,
            deposit_collateral,
            now_timestamp,
        );

        for (i, _user) in users.iter().enumerate() {
            assert_eq!(users[i].deposited_collateral.sol as u64, deposit_collateral);
            assert_eq!(users[i].borrowed_stablecoin, borrow_split.amount_to_borrow);
        }

        let total_deposited_amount = deposit_collateral * 2;
        let total_borrowed_amount = borrow_split.amount_to_borrow * 2;
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        let liquidator_fee = deposit_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let clearer_fee = deposit_collateral * (CLEARER_RATE as u64) / 10_000;

        let amount_to_withdraw = SOL::from(8.0); // user can withdraw more than initial deposit

        borrowing_operations::withdraw_collateral(
            &mut market,
            &mut users[1],
            amount_to_withdraw,
            CollateralToken::SOL,
            &TokenPrices::new(40.0),
        )
        .unwrap();

        // Liquidated user
        assert_eq!(users[0].borrowed_stablecoin, 0);
        assert_eq!(deposited!(users[0], CollateralToken::SOL), 0);

        // Redistributed user
        assert_fuzzy_eq!(users[1].borrowed_stablecoin, total_borrowed_amount, 2);
        assert_fuzzy_eq!(
            users[1].deposited_collateral.sol as u64,
            total_deposited_amount - amount_to_withdraw - liquidator_fee - clearer_fee,
            2
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount - amount_to_withdraw - liquidator_fee - clearer_fee
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
    }

    #[test]
    fn test_borrowing_withdraw_max_collateral_after_liquidation_and_redistribution() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let amount_to_borrow = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);

        let deposit_collateral = SOL::from(15.0);

        let mut users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            2,
            amount_to_borrow,
            deposit_collateral,
            now_timestamp,
        );

        for (_i, _user) in users.iter().enumerate() {
            assert_eq!(_user.deposited_collateral.sol as u64, deposit_collateral);
            assert_eq!(_user.borrowed_stablecoin, borrow_split.amount_to_borrow);
        }

        let total_deposited_amount = deposit_collateral * 2;
        let total_borrowed_amount = borrow_split.amount_to_borrow * 2;
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);

        let max_withdrawable_pre_liquidation_lamports = utils::calculate_max_withdrawable(
            &TokenPrices::new(40.0),
            &users[1],
            CollateralToken::SOL,
        );
        let amount_to_withdraw = max_withdrawable_pre_liquidation_lamports + 2;
        let withdraw_collateral_error = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut users[1],
            amount_to_withdraw,
            CollateralToken::SOL,
            &TokenPrices::new(40.0),
        );

        assert_eq!(
            withdraw_collateral_error.err(),
            Some(BorrowError::NotEnoughCollateral.into())
        );
        assert_eq!(deposited!(market, CollateralToken::SOL), SOL::from(30.0));
        assert_eq!(market.stablecoin_borrowed, USDH::from(402.0));
        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        for (_i, user) in users.iter_mut().enumerate() {
            borrowing_operations::refresh_positions(&mut market, user).unwrap();
        }

        let liquidator_fee = deposit_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let clearer_fee = deposit_collateral * (CLEARER_RATE as u64) / 10_000;

        let max_withdrawable_post_liquidation_lamports = utils::calculate_max_withdrawable(
            &TokenPrices::new(40.0),
            &users[1],
            CollateralToken::SOL,
        );

        assert!(max_withdrawable_post_liquidation_lamports > max_withdrawable_pre_liquidation_lamports,
            "Expected to be able to withdraw more collateral after liquidation and redistribution than before.");

        borrowing_operations::withdraw_collateral(
            &mut market,
            &mut users[1],
            amount_to_withdraw,
            CollateralToken::SOL,
            &TokenPrices::new(40.0),
        )
        .unwrap();

        // Liquidated user
        assert_eq!(users[0].borrowed_stablecoin, 0);
        assert_eq!(deposited!(users[0], CollateralToken::SOL), 0);

        // Redistributed user
        assert_fuzzy_eq!(users[1].borrowed_stablecoin, total_borrowed_amount, 2);
        assert_fuzzy_eq!(
            users[1].deposited_collateral.sol as u64,
            total_deposited_amount - amount_to_withdraw - liquidator_fee - clearer_fee,
            2
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount - amount_to_withdraw - liquidator_fee - clearer_fee
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
    }

    #[test]
    fn test_borrowing_three_liquidations() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        let liquidations = RefCell::new(LiquidationsQueue::default());

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let borrow_per_user = USDH::from(1000.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let debt_per_user = borrow_split.amount_to_borrow;
        let deposit_collateral = SOL::from(150.0);

        let num_borrowers = 3;
        let mut first_users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            borrow_per_user,
            deposit_collateral,
            now_timestamp,
        );
        let liquidator = Pubkey::new_unique();
        // First liquidation
        let first_liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut first_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        let mut second_users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            borrow_per_user,
            deposit_collateral,
            now_timestamp,
        );

        // Second liquidation
        let liquidator = Pubkey::new_unique();
        let second_liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut second_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        let mut third_users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            borrow_per_user,
            deposit_collateral,
            now_timestamp,
        );

        // Third liquidation
        let liquidator = Pubkey::new_unique();
        let third_liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut third_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        let liquidator_fee = deposit_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let clearer_fee = deposit_collateral * (CLEARER_RATE as u64) / 10_000;
        assert_eq!(
            liquidator_fee,
            first_liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol
        );
        assert_eq!(
            liquidator_fee,
            second_liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol
        );
        assert_eq!(
            liquidator_fee,
            third_liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol
        );

        borrowing_operations::refresh_positions(&mut market, &mut first_users[1]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut second_users[1]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut third_users[1]).unwrap();

        // users

        // Before
        // user1       debt=1005       coll=150
        // user2       debt=1005       coll=150
        // user3       debt=1005       coll=150

        // After liquidate user 1
        // 150 * 0.995 = 149.25
        // 149.25 / 2 = 74.625
        // 1005 / 2 = 502.5
        // user1       debt=0          coll=0
        // user2       debt=1005+502.5 = 1507.5       coll=150+149.25 = 299.25
        // user3       debt=1005+502.5 = 1507.5       coll=150+149.25 = 299.25

        // After add new users
        // user1       debt=0          coll=0
        // user2       debt=1507.5     coll=299.25
        // user3       debt=1507.5     coll=299.25
        // user4       debt=1005       coll=150
        // user5       debt=1005       coll=150
        // user6       debt=1005       coll=150

        // After liquidate 4
        // total_stake = 1507.5 + 1507.5 + 1005 + 1005 = 5025
        // debt_2_3 1507.5 / 5025  = 0.3 * 1005 = 301.5
        // debt_5_6 1005 / 5025  = 0.2 * 1005 = 201
        // coll_2_3 149.25 * 0.3 = 44.775
        // coll_5_6 149.25 * 0.2 = 29.85
        // user1       debt=0                     coll=0
        // user2       debt=1507.5+301.5=1809     coll=299.25+44.775=344.025
        // user3       debt=1507.5+301.5=1809     coll=299.25+44.775=344.025
        // user4       debt=0                     coll=0
        // user5       debt=1005+201=1206         coll=150+29.85=179.85
        // user6       debt=1005+201=1206         coll=150+29.85=179.85

        // After add new users
        // user1       debt=0                     coll=0
        // user2       debt=1809                  coll=344.025
        // user3       debt=1809                  coll=344.025
        // user4       debt=0                     coll=0
        // user5       debt=1206                  coll=179.85
        // user6       debt=1206                  coll=179.85
        // user7       debt=1005                  coll=150
        // user8       debt=1005                  coll=150
        // user9       debt=1005                  coll=150

        // After liquidate 7
        // total_stake = 1809*2 + 1206*2 + 1005*2 = 8040
        // debt_2_3 = 1809/8040 = 0.225 * 1005 = 226.125
        // debt_5_6 = 1206/8040 = 0.15 * 1005 = 150.75
        // debt_8_9 = 1005/8040 = 0.125 * 1005 = 125.625
        // coll_2_3 = 1809/8040 = 0.225 * 149.25 = 33.58125
        // coll_5_6 = 1206/8040 = 0.15 * 149.25 = 22.3875
        // coll_8_9 = 1005/8040 = 0.125 * 149.25 = 18.65625
        // user1       debt=0                     coll=0
        // user2       debt=1809+226.125=2035.125 coll=344.025+33.58125=377.60625
        // user3       debt=1809+226.125=2035.125 coll=344.025+33.58125=377.60625
        // user4       debt=0                     coll=0
        // user5       debt=1206+150.75=1356.75   coll=179.85+22.3875=202.23749
        // user6       debt=1206+150.75=1356.75   coll=179.85+22.3875=202.23749
        // user7       debt=0                     coll=0
        // user8       debt=1005+125.625=1130.625 coll=150+18.65625=168.65625
        // user9       debt=1005+125.625=1130.625 coll=150+18.65625=168.65625

        let coll_gains_each_liq = (deposit_collateral - liquidator_fee - clearer_fee) as u128;

        let total_stake_after_first_liq = (num_borrowers as u64 - 1) * debt_per_user;
        let user_1_first_liq_stake = debt_per_user;
        let user_1_first_liq_stablecoin_gain_per_user =
            debt_per_user * user_1_first_liq_stake / total_stake_after_first_liq;
        let user_1_first_liq_coll_gain_per_user = coll_gains_each_liq
            * (user_1_first_liq_stake as u128)
            / (total_stake_after_first_liq as u128);

        let total_stake_after_second_liq = (num_borrowers as u64 * 2 - 1) * debt_per_user;
        let user_1_second_liq_stake = debt_per_user + user_1_first_liq_stablecoin_gain_per_user;
        let user_1_second_liq_stablecoin_gain_per_user =
            debt_per_user * user_1_second_liq_stake / total_stake_after_second_liq;
        let user_1_second_liq_coll_gain_per_user = coll_gains_each_liq
            * (user_1_second_liq_stake as u128)
            / (total_stake_after_second_liq as u128);

        let total_stake_after_third_liq = (num_borrowers as u64 * 3 - 1) * debt_per_user;
        let user_1_third_liq_stake = debt_per_user
            + user_1_first_liq_stablecoin_gain_per_user
            + user_1_second_liq_stablecoin_gain_per_user;
        let user_1_third_liq_stablecoin_gain_per_user =
            debt_per_user * user_1_third_liq_stake / total_stake_after_third_liq;
        let user_1_third_liq_coll_gain_per_user = coll_gains_each_liq
            * (user_1_third_liq_stake as u128)
            / (total_stake_after_third_liq as u128);

        println!(
            "first {} second {} third {}",
            total_stake_after_first_liq, total_stake_after_second_liq, total_stake_after_third_liq,
        );

        println!(
            "first {} second {} third {}",
            user_1_first_liq_stablecoin_gain_per_user,
            user_1_second_liq_stablecoin_gain_per_user,
            user_1_third_liq_stablecoin_gain_per_user,
        );

        assert_fuzzy_eq!(
            first_users[1].borrowed_stablecoin,
            debt_per_user
                + user_1_first_liq_stablecoin_gain_per_user
                + user_1_second_liq_stablecoin_gain_per_user
                + user_1_third_liq_stablecoin_gain_per_user,
            2
        );

        assert_fuzzy_eq!(
            first_users[1].deposited_collateral.sol as u64,
            deposit_collateral
                + (user_1_first_liq_coll_gain_per_user as u64)
                + (user_1_second_liq_coll_gain_per_user as u64)
                + (user_1_third_liq_coll_gain_per_user as u64),
            6
        );

        let user_4_second_liq_stake = debt_per_user;
        let user_4_second_liq_stablecoin_gain_per_user =
            debt_per_user * user_4_second_liq_stake / total_stake_after_second_liq;
        let user_4_second_liq_coll_gain_per_user = coll_gains_each_liq
            * (user_4_second_liq_stake as u128)
            / (total_stake_after_second_liq as u128);

        let user_4_third_liq_stake = debt_per_user + user_4_second_liq_stablecoin_gain_per_user;
        let user_4_third_liq_stablecoin_gain_per_user =
            debt_per_user * user_4_third_liq_stake / total_stake_after_third_liq;
        let user_4_third_liq_coll_gain_per_user = coll_gains_each_liq
            * (user_4_third_liq_stake as u128)
            / (total_stake_after_third_liq as u128);

        assert_eq!(
            second_users[1].borrowed_stablecoin,
            debt_per_user
                + user_4_second_liq_stablecoin_gain_per_user
                + user_4_third_liq_stablecoin_gain_per_user
        );
        assert_fuzzy_eq!(
            second_users[1].deposited_collateral.sol as u64,
            deposit_collateral
                + (user_4_second_liq_coll_gain_per_user as u64)
                + (user_4_third_liq_coll_gain_per_user as u64),
            4
        );

        let user_7_third_liq_stake = debt_per_user;
        let user_7_third_liq_stablecoin_gain_per_user =
            debt_per_user * user_7_third_liq_stake / total_stake_after_third_liq;
        let user_7_third_liq_coll_gain_per_user = coll_gains_each_liq
            * (user_7_third_liq_stake as u128)
            / (total_stake_after_third_liq as u128);

        assert_fuzzy_eq!(
            third_users[1].borrowed_stablecoin,
            debt_per_user + user_7_third_liq_stablecoin_gain_per_user,
            2
        );
        assert_fuzzy_eq!(
            third_users[1].deposited_collateral.sol as u64,
            deposit_collateral + (user_7_third_liq_coll_gain_per_user as u64),
            10
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            (deposit_collateral * (3 * num_borrowers as u64)
                - ((liquidator_fee + clearer_fee) * 3))
        );
        assert_eq!(
            market.stablecoin_borrowed,
            debt_per_user * (3 * num_borrowers as u64)
        );
    }

    #[test]
    fn test_borrowing_sequenced_liquidations_and_borrowings() {
        // Here, we liquidate immediately after the user has deposited & borrowed
        // ensuring the next liqudation generates a higher redistribution

        // The difference between this test and the one above is that here we don't "refresh"
        // each user's position to harvest the rewards after every single liquidation,
        // but only rather do it at the end behaviour being the same ensures users
        // can stake/unstake/harvest whenever they wish

        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();

        let liquidations = RefCell::new(LiquidationsQueue::default());

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let borrow_per_user = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let deposit_collateral = SOL::from(15.0);
        let debt_per_user = borrow_split.amount_to_borrow;

        let extra_borrow_per_user = USDH::from(5.0);
        let extra_stablecoin_borrow =
            BorrowSplit::from_amount(extra_borrow_per_user, market.base_rate_bps);
        let extra_collateral_deposit = SOL::from(3.3);
        let extra_debt_per_user = extra_stablecoin_borrow.amount_to_borrow;

        let num_borrowers = 10;
        let mut borrowing_users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            borrow_per_user,
            deposit_collateral,
            now_timestamp,
        );

        // First liquidation
        let liquidator = Pubkey::new_unique();
        let first_liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        // First liquidation: Deposit & Borrow
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut borrowing_users[1],
            extra_collateral_deposit,
            CollateralToken::SOL,
        )
        .unwrap();

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut borrowing_users[1],
            &mut staking_pool_state,
            extra_stablecoin_borrow.amount_to_borrow - extra_stablecoin_borrow.fees_to_pay,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        // Second liquidation
        let liquidator = Pubkey::new_unique();
        let second_liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[1],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        // Second liquidation: Deposit & Borrow
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut borrowing_users[2],
            extra_collateral_deposit,
            CollateralToken::SOL,
        )
        .unwrap();

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut borrowing_users[2],
            &mut staking_pool_state,
            extra_stablecoin_borrow.amount_to_borrow - extra_stablecoin_borrow.fees_to_pay,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        // Third liquidation
        let liquidator = Pubkey::new_unique();
        let third_liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[2],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        // Third liquidation: Deposit & Borrow
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut borrowing_users[3],
            extra_collateral_deposit,
            CollateralToken::SOL,
        )
        .unwrap();

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut borrowing_users[3],
            &mut staking_pool_state,
            extra_stablecoin_borrow.amount_to_borrow - extra_stablecoin_borrow.fees_to_pay,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        // liquidated user[0]
        let first_liquidated_user_collateral = deposit_collateral;
        let first_liquidator_fee =
            first_liquidated_user_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let first_clearer_fee = first_liquidated_user_collateral * (CLEARER_RATE as u64) / 10_000;
        let first_liquidation_collateral_gain_per_user =
            (first_liquidated_user_collateral - first_liquidator_fee - first_clearer_fee) / 9;
        let first_liquidation_stablecoin_gain_per_user = debt_per_user / 9;

        // liquidated user[1]
        let second_liquidated_user_collateral = deposit_collateral
            + first_liquidation_collateral_gain_per_user
            + extra_collateral_deposit;

        let second_liquidator_fee =
            second_liquidated_user_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let second_clearer_fee = second_liquidated_user_collateral * (CLEARER_RATE as u64) / 10_000;
        let second_liquidation_collateral_gain_per_user =
            (second_liquidated_user_collateral - second_liquidator_fee - second_clearer_fee) / 8;
        let second_liquidation_stablecoin_gain_per_user =
            (debt_per_user + first_liquidation_stablecoin_gain_per_user + extra_debt_per_user) / 8;

        // liquidated user[2]
        let third_liquidated_user_collateral = deposit_collateral
            + first_liquidation_collateral_gain_per_user
            + second_liquidation_collateral_gain_per_user
            + extra_collateral_deposit;

        let third_liquidator_fee =
            third_liquidated_user_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let third_clearer_fee = third_liquidated_user_collateral * (CLEARER_RATE as u64) / 10_000;
        let third_liquidation_collateral_gain_per_user =
            (third_liquidated_user_collateral - third_liquidator_fee - third_clearer_fee) / 7;
        let third_liquidation_stablecoin_gain_per_user = (debt_per_user
            + first_liquidation_stablecoin_gain_per_user
            + second_liquidation_stablecoin_gain_per_user
            + extra_debt_per_user)
            / 7;

        assert_eq!(
            first_liquidator_fee,
            first_liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol as u64
        );
        assert_eq!(
            second_liquidator_fee,
            second_liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol as u64
        );
        assert_eq!(
            third_liquidator_fee,
            third_liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol as u64
        );

        // liquidate user[0]
        // Start with 15000000000 (before liquidation 0)
        // After first liquidation 16658333333 -> 16658333333 - 15000000000 = 1658333333
        // first liquidation liquidator gain: 15000000000 * 0.005 = 75000000
        // first liquidation distributed gain: 15000000000 - 75000000 = 14925000000 / 9  = 1658333333.3333333
        // bef liquidate user[0] -> user[1] = 15000000000
        // aft liquidate user[0] -> user[1] = 16658333333
        // bef liquidate user[0] -> user[2] = 15000000000
        // aft liquidate user[0] -> user[2] = 16658333333
        // bef liquidate user[0] -> user[3] = 15000000000
        // aft liquidate user[0] -> user[3] = 16658333333
        // bef liquidate user[0] -> user[4] = 15000000000
        // aft liquidate user[0] -> user[4] = 16658333333

        // second liquidation user before liquidation 19958333333
        // 19958333333 - 16658333333 = 3300000000
        // second liquidation liquidator gain: 19958333333 * 0.005 = 99791666.665
        // second liquidation distributed gain: 19958333333 - 99791666 = 19858541667
        // second liquidation distributed gain per user 19858541667 / 8 = 2482317708.375

        // third liquidation user before liquidation 22440651041
        // 22440651041 - 19140651041 = 3300000000
        // third liquidation liquidator gain: 22440651041 * 0.005 = 112203255.205
        // third liquidation distributed gain: 22440651041 - 112203255.205 = 22328447785.795
        // third liquidation distributed gain per user :  22328447785.795 / 7  = 3189778255.113571
        // 19140651041 + 3189778255.113571 = 22330429296.11357

        for i in 0..10 {
            borrowing_operations::refresh_positions(&mut market, &mut borrowing_users[i]).unwrap();
        }

        assert_eq!(borrowing_users[0].borrowed_stablecoin, 0);
        assert_eq!(borrowing_users[1].borrowed_stablecoin, 0);
        assert_eq!(borrowing_users[2].borrowed_stablecoin, 0);

        assert_eq!(deposited!(borrowing_users[0], CollateralToken::SOL), 0);
        assert_eq!(borrowing_users[1].deposited_collateral.sol as u64, 0);
        assert_eq!(borrowing_users[2].deposited_collateral.sol as u64, 0);

        assert_eq!(
            borrowing_users[3].deposited_collateral.sol as u64,
            deposit_collateral
                + first_liquidation_collateral_gain_per_user
                + second_liquidation_collateral_gain_per_user
                + third_liquidation_collateral_gain_per_user
                + extra_collateral_deposit
        );

        assert_fuzzy_eq!(
            borrowing_users[3].borrowed_stablecoin,
            debt_per_user
                + extra_debt_per_user
                + first_liquidation_stablecoin_gain_per_user
                + second_liquidation_stablecoin_gain_per_user
                + third_liquidation_stablecoin_gain_per_user,
            3
        );

        assert_eq!(
            borrowing_users[4].deposited_collateral.sol as u64,
            deposit_collateral
                + first_liquidation_collateral_gain_per_user
                + second_liquidation_collateral_gain_per_user
                + third_liquidation_collateral_gain_per_user
        );

        assert_fuzzy_eq!(
            borrowing_users[4].borrowed_stablecoin,
            debt_per_user
                + first_liquidation_stablecoin_gain_per_user
                + second_liquidation_stablecoin_gain_per_user
                + third_liquidation_stablecoin_gain_per_user,
            3
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            (deposit_collateral * 10 + extra_collateral_deposit * 3
                - first_liquidator_fee
                - second_liquidator_fee
                - first_clearer_fee
                - second_clearer_fee
                - third_liquidator_fee
                - third_clearer_fee)
        );
        assert_eq!(
            market.stablecoin_borrowed,
            debt_per_user * 10 + extra_debt_per_user * 3
        );
    }

    #[test]
    fn test_borrowing_liquidate_after_distribution_without_applying_pending_rewards() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        let liquidations = RefCell::new(LiquidationsQueue::default());

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let borrow_per_user = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let debt_per_user = borrow_split.amount_to_borrow;
        let deposit_collateral = SOL::from(15.0);
        let extra_collateral_deposit = SOL::from(3.3);

        let num_borrowers = 10;
        let mut borrowing_users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            borrow_per_user,
            deposit_collateral,
            now_timestamp,
        );

        // First liquidation
        let liquidator = Pubkey::new_unique();
        let first_liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        // no refresh here..

        // Second liquidation
        let second_liquidation_effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[1],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        // Third liquidation: Deposit & Borrow
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut borrowing_users[2],
            extra_collateral_deposit,
            CollateralToken::SOL,
        )
        .unwrap();

        // liquidated user[0]
        let first_liquidated_user_collateral = deposit_collateral;
        let first_liquidator_fee =
            first_liquidated_user_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let first_clearer_fee = first_liquidated_user_collateral * (CLEARER_RATE as u64) / 10_000;
        let first_liquidation_collateral_gain_per_user =
            (first_liquidated_user_collateral - first_liquidator_fee - first_clearer_fee) / 9;
        let first_liquidation_stablecoin_gain_per_user = debt_per_user / 9;

        // liquidated user[1]
        let second_liquidated_user_collateral =
            deposit_collateral + first_liquidation_collateral_gain_per_user;

        let second_liquidator_fee =
            second_liquidated_user_collateral * (LIQUIDATOR_RATE as u64) / 10_000;
        let second_clearer_fee = second_liquidated_user_collateral * (CLEARER_RATE as u64) / 10_000;
        let second_liquidation_collateral_gain_per_user =
            (second_liquidated_user_collateral - second_liquidator_fee - second_clearer_fee) / 8;
        let second_liquidation_stablecoin_gain_per_user =
            (debt_per_user + first_liquidation_stablecoin_gain_per_user) / 8;

        assert_eq!(
            first_liquidator_fee,
            first_liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol
        );
        assert_eq!(
            second_liquidator_fee,
            second_liquidation_effects
                .liquidation_event
                .collateral_gain_to_liquidator
                .sol
        );
        assert_eq!(
            first_clearer_fee,
            first_liquidation_effects
                .liquidation_event
                .collateral_gain_to_clearer
                .sol
        );
        assert_eq!(
            second_clearer_fee,
            second_liquidation_effects
                .liquidation_event
                .collateral_gain_to_clearer
                .sol
        );

        for i in 0..10 {
            borrowing_operations::refresh_positions(&mut market, &mut borrowing_users[i]).unwrap();
        }

        assert_eq!(borrowing_users[0].borrowed_stablecoin, 0);
        assert_eq!(borrowing_users[1].borrowed_stablecoin, 0);

        assert_eq!(deposited!(borrowing_users[0], CollateralToken::SOL), 0);
        assert_eq!(borrowing_users[1].deposited_collateral.sol as u64, 0);

        assert_eq!(
            borrowing_users[2].deposited_collateral.sol as u64,
            deposit_collateral
                + first_liquidation_collateral_gain_per_user
                + second_liquidation_collateral_gain_per_user
                + extra_collateral_deposit
        );

        assert_fuzzy_eq!(
            borrowing_users[2].borrowed_stablecoin,
            debt_per_user
                + first_liquidation_stablecoin_gain_per_user
                + second_liquidation_stablecoin_gain_per_user,
            2
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            (deposit_collateral * 10 + extra_collateral_deposit * 1
                - first_liquidator_fee
                - second_liquidator_fee
                - first_clearer_fee
                - second_clearer_fee)
        );
        assert_eq!(market.stablecoin_borrowed, debt_per_user * 10);
    }

    #[test]
    fn test_borrowing_liquidate_until_there_is_only_one_left() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let now_timestamp = 0;
        let hbb_emissions_start_ts = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let borrow_per_user = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let debt_per_user = borrow_split.amount_to_borrow;
        let deposit_collateral = SOL::from(15.0);

        // 100 debt
        // 15 * 40 = 600 collateral
        // nothing in the stability pool -> redistributing everything
        // max debt: 150 (recovery mode)
        //

        let num_borrowers = 10;
        let mut borrowing_users = utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            borrow_per_user,
            deposit_collateral,
            now_timestamp,
        );

        // Liquidate all but last user
        let agents_gains: Vec<(u64, u64)> = borrowing_users[0..(num_borrowers - 1)]
            .iter_mut()
            .map(|user| {
                let liquidator = Pubkey::new_unique();
                let liq_event = borrowing_operations::try_liquidate(
                    liquidator,
                    &mut market,
                    user,
                    &mut stability_pool_state,
                    &mut epoch_to_scale_to_sum,
                    &TokenPrices::new(0.1),
                    &mut liquidations.borrow_mut(),
                    0,
                )
                .unwrap()
                .liquidation_event;

                (
                    liq_event.collateral_gain_to_liquidator.sol as u64,
                    liq_event.collateral_gain_to_clearer.sol as u64,
                )
            })
            .collect();

        let liquidation_gains: u64 = agents_gains.iter().map(|x| x.0).sum();
        let clearing_gains: u64 = agents_gains.iter().map(|x| x.1).sum();

        let mut accum_distributed_liquidation = 0;
        let mut accum_liquidator_fee = 0;
        for i in 0..(num_borrowers - 1) {
            let current_user_collateral = deposit_collateral + accum_distributed_liquidation;
            let current_user_liq_fee = LIQUIDATOR_RATE as u64 * current_user_collateral / 10_000;
            let current_user_clear_fee = CLEARER_RATE as u64 * current_user_collateral / 10_000;
            let current_user_redistributed_coll =
                current_user_collateral - current_user_liq_fee - current_user_clear_fee;

            accum_distributed_liquidation +=
                current_user_redistributed_coll / ((num_borrowers - 1 - i) as u64);
            accum_liquidator_fee += current_user_liq_fee;
        }
        assert_eq!(liquidation_gains, accum_liquidator_fee);

        // Attempt to liquidate the last user
        let liquidator = Pubkey::new_unique();
        let res = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[num_borrowers - 1],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(0.1),
            &mut liquidations.borrow_mut(),
            0,
        );
        assert!(res.is_err());

        for i in 0..num_borrowers {
            borrowing_operations::refresh_positions(&mut market, &mut borrowing_users[i]).unwrap();
        }

        // Assert all but last user have 0
        for i in 0..(num_borrowers - 1) {
            assert_eq!(borrowing_users[i].borrowed_stablecoin, 0);
            assert_eq!(borrowing_users[i].deposited_collateral.sol as u64, 0);
        }

        // Assert last user has all less the liquidator fee
        // and all of the stablecoin

        let remaining_col: u64 =
            deposit_collateral * (num_borrowers as u64) - liquidation_gains - clearing_gains;
        let remaining_stable = debt_per_user * num_borrowers as u64;
        assert_fuzzy_eq!(
            borrowing_users[num_borrowers - 1].borrowed_stablecoin,
            remaining_stable,
            11
        );
        assert_fuzzy_eq!(
            borrowing_users[num_borrowers - 1].deposited_collateral.sol as u64,
            remaining_col,
            10
        );

        // Market
        assert_eq!(deposited!(market, CollateralToken::SOL), remaining_col);
        assert_eq!(market.stablecoin_borrowed, remaining_stable);
    }
}
