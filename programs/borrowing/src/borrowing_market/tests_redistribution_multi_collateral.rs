#![allow(unaligned_references)]
#[cfg(test)]
mod tests {

    const SE: u64 = 10;

    use std::cell::RefCell;

    use anchor_lang::{prelude::Pubkey, solana_program::native_token::sol_to_lamports};
    use decimal_wad::ratio::Ratio;

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
        state::epoch_to_scale_to_sum::EpochToScaleToSum,
        utils::{
            consts::{CLEARER_RATE, LIQUIDATOR_RATE, ONE},
            coretypes::USDH,
            math::coll_to_lamports,
        },
        BorrowingMarketState, CollateralAmounts, CollateralToken, LiquidationsQueue, Price,
        StabilityPoolState, StakingPoolState, TokenPrices, UserMetadata,
    };

    #[test]
    fn test_borrowing_liquidate_redistribution_multi_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let deposits_lamports = CollateralAmounts {
            sol: coll_to_lamports(15.0, CollateralToken::SOL),
            eth: coll_to_lamports(10.0, CollateralToken::ETH),
            btc: coll_to_lamports(7.6, CollateralToken::BTC),
            ftt: coll_to_lamports(8.3, CollateralToken::FTT),
            ..Default::default()
        };

        let liquidation_prices = 1.0;
        let borrow_per_user = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let borrow_prices = TokenPrices::new_all(liquidation_prices + 100.0);
        let _liq_prices = TokenPrices::new_all(liquidation_prices);

        let mut total_amount_borrowed = 0;
        let mut total_amount_deposited = CollateralAmounts::default();

        let count = 10;
        let mut users: Vec<UserMetadata> = (0..count)
            .map(|_| {
                let mut user = UserMetadata::default();
                borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

                use CollateralToken::*;
                for token in [SOL, ETH, SRM, FTT, BTC, RAY] {
                    let amount = deposits_lamports.token_amount(token);
                    if amount > 0 {
                        borrowing_operations::deposit_collateral(
                            &mut market,
                            &mut user,
                            amount as u64,
                            token,
                        )
                        .map_err(|e| {
                            println!("Error depositing {:?}", e);
                            e
                        })
                        .unwrap();
                    }
                }

                total_amount_deposited = total_amount_deposited.add(&deposits_lamports);

                borrowing_operations::borrow_stablecoin(
                    &mut market,
                    &mut user,
                    &mut staking_pool_state,
                    borrow_per_user,
                    &borrow_prices,
                    now_timestamp,
                )
                .unwrap();

                total_amount_borrowed += borrow_split.amount_to_borrow;

                user
            })
            .collect();

        println!("{}", market.to_state_string());

        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices {
                sol: Price::from_f64(1.0, CollateralToken::SOL),
                eth: Price::from_f64(1.0, CollateralToken::ETH),
                btc: Price::from_f64(1.0, CollateralToken::BTC),
                srm: Price::from_f64(1.0, CollateralToken::SRM),
                ray: Price::from_f64(1.0, CollateralToken::RAY),
                ftt: Price::from_f64(1.0, CollateralToken::FTT),
            },
            &mut liquidations.borrow_mut(),
            0,
        )
        .unwrap();

        // There are 10 users with
        // - 1.000.000 stablecoin borrowed each
        // - 10000000000 sol deposited each

        // After 1st user gets liquidated (MINUS COLL FEE LIQUIDATOR_RATE) ->
        // 1.000.000 is split in 9 = 1000000  / 9 = 111111.11111111111 (RPT)
        // 9950000000 is split in 9 = 9950000000 / 9 = 1105555555.5555556 (RPT)
        // minus liquidation fee
        // expected coll each: 10000000000 + 1105555555 = 11105555555
        // expected debt each: 1000000 + 111111 = 1111111

        borrowing_operations::refresh_positions(&mut market, &mut users[0]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut users[1]).unwrap();

        let liquidator_fees = deposits_lamports.mul_bps(LIQUIDATOR_RATE);
        let clearer_fees = deposits_lamports.mul_bps(CLEARER_RATE);
        println!("Liquidator fees {:?}", liquidator_fees);

        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            (total_amount_deposited.sol - liquidator_fees.sol - clearer_fees.sol) as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::ETH),
            (total_amount_deposited.eth - liquidator_fees.eth - clearer_fees.eth) as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::BTC),
            (total_amount_deposited.btc - liquidator_fees.btc - clearer_fees.btc) as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::SRM),
            (total_amount_deposited.srm - liquidator_fees.srm - clearer_fees.srm) as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::FTT),
            (total_amount_deposited.ftt - liquidator_fees.ftt - clearer_fees.ftt) as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::RAY),
            (total_amount_deposited.ray - liquidator_fees.ray - clearer_fees.ray) as u64
        );

        // Liquidated user
        assert_eq!(users[0].borrowed_stablecoin, 0);
        assert_eq!(deposited!(users[0], CollateralToken::SOL), 0);

        // Redistributed user
        let updated_borrowed_usd_per_user =
            borrow_split.amount_to_borrow + borrow_split.amount_to_borrow / ((count - 1) as u64);
        assert_eq!(users[1].borrowed_stablecoin, updated_borrowed_usd_per_user);

        let updated_deposited_coll = total_amount_deposited
            .sub(&liquidator_fees)
            .sub(&clearer_fees)
            .div_scalar(count - 1);

        println!("Total amount deposited col {:?}", total_amount_deposited);
        println!("Updated deposited col {:?}", updated_deposited_coll);
        println!("Liquidator fees {:?}", liquidator_fees);

        // 83000000000 - 41500000 = 82958500000 / 9 = 9217611111.11111 = 9222222222.222221

        assert_eq!(
            deposited!(users[1], CollateralToken::SOL),
            updated_deposited_coll.sol as u64
        );
        assert_eq!(
            deposited!(users[1], CollateralToken::ETH),
            updated_deposited_coll.eth as u64
        );
        assert_eq!(
            deposited!(users[1], CollateralToken::BTC),
            updated_deposited_coll.btc as u64
        );
        assert_eq!(
            deposited!(users[1], CollateralToken::SRM),
            updated_deposited_coll.srm as u64
        );
        assert_eq!(
            deposited!(users[1], CollateralToken::FTT),
            updated_deposited_coll.ftt as u64
        );
        assert_eq!(
            deposited!(users[1], CollateralToken::RAY),
            updated_deposited_coll.ray as u64
        );

        // Market
        let remaining_deposited_coll = total_amount_deposited
            .sub(&liquidator_fees)
            .sub(&clearer_fees);
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            remaining_deposited_coll.sol as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::ETH),
            remaining_deposited_coll.eth as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::BTC),
            remaining_deposited_coll.btc as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::SRM),
            remaining_deposited_coll.srm as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::FTT),
            remaining_deposited_coll.ftt as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::RAY),
            remaining_deposited_coll.ray as u64
        );

        assert_eq!(market.stablecoin_borrowed, total_amount_borrowed);
    }

    #[test]
    fn test_borrowing_two_users_fully_redistribute_multi_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let borrow_per_user = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);

        let deposits_lamports = CollateralAmounts {
            sol: coll_to_lamports(15.0, CollateralToken::SOL),
            eth: coll_to_lamports(10.0, CollateralToken::ETH),
            btc: coll_to_lamports(7.6, CollateralToken::BTC),
            ftt: coll_to_lamports(8.3, CollateralToken::FTT),
            ..Default::default()
        };

        let count = 2;
        let mut users = utils::new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            count,
            &vec![borrow_per_user; count],
            &vec![deposits_lamports; count],
            now_timestamp,
        );

        for (_i, _user) in users.iter().enumerate() {
            assert_eq!(
                deposited!(_user, CollateralToken::SOL),
                deposits_lamports.sol as u64
            );
            assert_eq!(_user.borrowed_stablecoin, borrow_split.amount_to_borrow);
        }

        let total_deposited_amount = deposits_lamports.mul_scalar(2);
        let total_borrowed_amount = borrow_split.amount_to_borrow * 2;

        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount.sol as u64
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

        let liquidator_fees = deposits_lamports.mul_bps(LIQUIDATOR_RATE);
        let clearer_fees = deposits_lamports.mul_bps(CLEARER_RATE);

        let remaining_deposited_coll = deposits_lamports
            .mul_scalar(count as u64)
            .sub(&liquidator_fees)
            .sub(&clearer_fees);

        // Liquidated user
        assert_eq!(users[0].borrowed_stablecoin, 0);
        assert_eq!(deposited!(users[0], CollateralToken::SOL), 0);

        // Redistributed user
        let updated_deposited_coll = remaining_deposited_coll.div_scalar((count - 1) as u64);
        assert_fuzzy_eq!(users[1].borrowed_stablecoin, total_borrowed_amount, 2);

        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::SOL),
            updated_deposited_coll.sol,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::ETH),
            updated_deposited_coll.eth,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::BTC),
            updated_deposited_coll.btc,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::SRM),
            updated_deposited_coll.srm,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::FTT),
            updated_deposited_coll.ftt,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::RAY),
            updated_deposited_coll.ray,
            2
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            remaining_deposited_coll.sol as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::ETH),
            remaining_deposited_coll.eth as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::BTC),
            remaining_deposited_coll.btc as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::SRM),
            remaining_deposited_coll.srm as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::FTT),
            remaining_deposited_coll.ftt as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::RAY),
            remaining_deposited_coll.ray as u64
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
    }

    #[test]
    fn test_borrowing_three_users_fully_redistribute_based_on_usd_amounts_multi_collateral() {
        // Upon redistribution debt and collateral is reallocated based on the user's usd deposits
        // If a user has 75% of the whole pool of usd, then they get 75% of the redistributed col and debt
        // the other users take the remaining 25% of the redistributed usd and coll
        // In this test we have one user having 1/3 of the pool and the other 2/3 of the pool

        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let borrow_per_user = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);

        let deposits_lamports = CollateralAmounts {
            sol: coll_to_lamports(15.0, CollateralToken::SOL),
            eth: coll_to_lamports(10.0, CollateralToken::ETH),
            btc: coll_to_lamports(7.6, CollateralToken::BTC),
            ftt: coll_to_lamports(8.3, CollateralToken::FTT),
            ..Default::default()
        };

        // the user to be liquidated
        let mut borrowers = utils::new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            1,
            &vec![borrow_per_user; 1],
            &vec![deposits_lamports; 1],
            now_timestamp,
        );

        // the users to gain liquidations
        let borrow_splits = [
            BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps),
            BorrowSplit::from_amount(borrow_per_user * 2, market.base_rate_bps),
        ];

        let mut users = utils::new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            2,
            &vec![borrow_per_user, borrow_per_user * 2],
            &vec![deposits_lamports; 2],
            now_timestamp,
        );

        // the user to be liquidated
        assert_eq!(
            deposited!(users[0], CollateralToken::SOL),
            deposits_lamports.sol as u64
        );
        assert_eq!(
            deposited!(users[0], CollateralToken::ETH),
            deposits_lamports.eth as u64
        );
        assert_eq!(
            deposited!(users[0], CollateralToken::BTC),
            deposits_lamports.btc as u64
        );
        assert_eq!(
            deposited!(users[0], CollateralToken::SRM),
            deposits_lamports.srm as u64
        );
        assert_eq!(
            deposited!(users[0], CollateralToken::FTT),
            deposits_lamports.ftt as u64
        );
        assert_eq!(
            deposited!(users[0], CollateralToken::RAY),
            deposits_lamports.ray as u64
        );

        assert_eq!(
            borrowers[0].borrowed_stablecoin,
            borrow_split.amount_to_borrow
        );

        // the users to gain liquidations
        for (i, _user) in users.iter().enumerate() {
            assert_eq!(
                deposited!(_user, CollateralToken::SOL),
                deposits_lamports.sol as u64
            );
            assert_eq!(_user.borrowed_stablecoin, borrow_splits[i].amount_to_borrow);
        }

        let total_deposited_amount = deposits_lamports.mul_scalar(3);
        let total_borrowed_amount = borrow_split.amount_to_borrow * 4;

        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount.sol as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::ETH),
            total_deposited_amount.eth as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::BTC),
            total_deposited_amount.btc as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::SRM),
            total_deposited_amount.srm as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::FTT),
            total_deposited_amount.ftt as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::RAY),
            total_deposited_amount.ray as u64
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);
        let liquidator = Pubkey::new_unique();

        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowers[0],
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

        let liquidator_fees = deposits_lamports.mul_bps(LIQUIDATOR_RATE);
        let clearer_fees = deposits_lamports.mul_bps(CLEARER_RATE);
        let remaining_deposited_coll = total_deposited_amount
            .sub(&liquidator_fees)
            .sub(&clearer_fees);

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            remaining_deposited_coll.sol as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::ETH),
            remaining_deposited_coll.eth as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::BTC),
            remaining_deposited_coll.btc as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::SRM),
            remaining_deposited_coll.srm as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::FTT),
            remaining_deposited_coll.ftt as u64
        );
        assert_eq!(
            deposited!(market, CollateralToken::RAY),
            remaining_deposited_coll.ray as u64
        );
        assert_eq!(market.stablecoin_borrowed, total_borrowed_amount);

        // Liquidated user
        assert_eq!(borrowers[0].borrowed_stablecoin, 0);
        assert_eq!(deposited!(borrowers[0], CollateralToken::SOL), 0);

        let redistributed_debt = borrow_split.amount_to_borrow;
        let redistributed_coll = deposits_lamports.sub(&liquidator_fees).sub(&clearer_fees);

        // First user gets 1/3 of the debt and 1/3 of the collateral
        assert_fuzzy_eq!(
            users[0].borrowed_stablecoin,
            borrow_split.amount_to_borrow + redistributed_debt * 1 / 3,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[0], CollateralToken::SOL),
            deposits_lamports.sol + (redistributed_coll.sol * 1 / 3),
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[0], CollateralToken::ETH),
            deposits_lamports.eth + (redistributed_coll.eth * 1 / 3),
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[0], CollateralToken::BTC),
            deposits_lamports.btc + (redistributed_coll.btc * 1 / 3),
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[0], CollateralToken::SRM),
            deposits_lamports.srm + (redistributed_coll.srm * 1 / 3),
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[0], CollateralToken::RAY),
            deposits_lamports.ray + (redistributed_coll.ray * 1 / 3),
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[0], CollateralToken::FTT),
            deposits_lamports.ftt + (redistributed_coll.ftt * 1 / 3),
            2
        );

        // Second user gets 2/3 of the debt and 2/3 of the collateral
        assert_fuzzy_eq!(
            users[1].borrowed_stablecoin,
            borrow_split.amount_to_borrow * 2 + redistributed_debt * 2 / 3,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::SOL),
            (deposits_lamports.sol as f64 + ((redistributed_coll.sol as f64) * (2.0 / 3.0))) as u64,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::ETH),
            (deposits_lamports.eth as f64 + ((redistributed_coll.eth as f64) * (2.0 / 3.0))) as u64,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::BTC),
            (deposits_lamports.btc as f64 + ((redistributed_coll.btc as f64) * (2.0 / 3.0))) as u64,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::SRM),
            (deposits_lamports.srm as f64 + ((redistributed_coll.srm as f64) * (2.0 / 3.0))) as u64,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::FTT),
            (deposits_lamports.ftt as f64 + ((redistributed_coll.ftt as f64) * (2.0 / 3.0))) as u64,
            2
        );
        assert_fuzzy_eq!(
            deposited!(users[1], CollateralToken::RAY),
            (deposits_lamports.ray as f64 + ((redistributed_coll.ray as f64) * (2.0 / 3.0))) as u64,
            2
        );
    }

    #[test]
    fn test_borrowing_liquidation_split_between_stability_pool_and_redistribution_multi_collateral()
    {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;
        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let liquidation_prices = 100.0;
        let borrow_per_user = USDH::from(3700.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);

        let deposits_lamports = CollateralAmounts {
            sol: coll_to_lamports(15.0, CollateralToken::SOL),
            eth: coll_to_lamports(10.0, CollateralToken::ETH),
            btc: coll_to_lamports(7.6, CollateralToken::BTC),
            ftt: coll_to_lamports(8.3, CollateralToken::FTT),
            ..Default::default()
        };

        let num_borrowers = 10;
        let mut borrowing_users = utils::new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            &vec![borrow_per_user; num_borrowers],
            &vec![deposits_lamports; num_borrowers],
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
            1000.0,
        );
        let sp_usd_deposits: u64 = stability_providers
            .iter()
            .map(|s| s.deposited_stablecoin)
            .sum();

        println!("User before liquidation: {:?}", borrowing_users[0]);
        println!("Borrowing market before liq {}", market.to_state_string());
        println!("SP before liq {}", stability_pool_state.to_state_string());

        // Liquidation effects
        let liquidator = Pubkey::new_unique();

        let effects = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowing_users[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new_all(liquidation_prices),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        // Liquidator fees
        let liquidator_fees = deposits_lamports.mul_bps(LIQUIDATOR_RATE);
        let clearer_fees = deposits_lamports.mul_bps(CLEARER_RATE);

        assert_eq!(
            liquidator_fees.sol,
            effects.liquidation_event.collateral_gain_to_liquidator.sol
        );
        assert_eq!(
            liquidator_fees.eth,
            effects.liquidation_event.collateral_gain_to_liquidator.eth
        );
        assert_eq!(
            liquidator_fees.btc,
            effects.liquidation_event.collateral_gain_to_liquidator.btc
        );
        assert_eq!(
            liquidator_fees.srm,
            effects.liquidation_event.collateral_gain_to_liquidator.srm
        );
        assert_eq!(
            liquidator_fees.ftt,
            effects.liquidation_event.collateral_gain_to_liquidator.ftt
        );
        assert_eq!(
            liquidator_fees.ray,
            effects.liquidation_event.collateral_gain_to_liquidator.ray
        );

        println!("After liq {}", market.to_state_string());
        println!("After liq {}", stability_pool_state.to_state_string());

        // Before liquidation
        // User debt: 10050000
        // User collateral: 15000000000
        // Stability pool: 400000

        // Liquidator fee: LIQUIDATOR_RATE * 15000000000 = 75000000

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

        println!("SP {}", stability_providers[0].to_state_string());

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

        let remaining_deposited_coll = deposits_lamports.sub(&liquidator_fees).sub(&clearer_fees);

        let sp_ratio = Ratio::new(sp_usd_deposits, borrow_split.amount_to_borrow);
        let stability_pool_coll_absorbed =
            remaining_deposited_coll.mul_fraction(sp_ratio.numerator, sp_ratio.denominator);

        {
            println!("SP {}", stability_providers[0].to_state_string());

            let user_gains_pending = &stability_providers[0].pending_gains_per_user;
            let user_gains_cumulative = &stability_providers[0].cumulative_gains_per_user;
            let user_deposits = &stability_providers[0].deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;

            assert_eq!(user_gains_pending.sol as u64, 0);
            assert_eq!(user_gains_pending.eth as u64, 0);
            assert_eq!(user_gains_pending.btc as u64, 0);
            assert_eq!(user_gains_pending.srm as u64, 0);
            assert_eq!(user_gains_pending.ftt as u64, 0);
            assert_eq!(user_gains_pending.ray as u64, 0);

            assert_eq!(
                user_gains_cumulative.sol as u64,
                stability_pool_coll_absorbed.sol / 2
            );
            assert_eq!(
                user_gains_cumulative.eth as u64,
                stability_pool_coll_absorbed.eth / 2
            );
            assert_eq!(
                user_gains_cumulative.btc as u64,
                stability_pool_coll_absorbed.btc / 2
            );
            assert_eq!(
                user_gains_cumulative.srm as u64,
                stability_pool_coll_absorbed.srm / 2
            );
            assert_eq!(
                user_gains_cumulative.ftt as u64,
                stability_pool_coll_absorbed.ftt / 2
            );
            assert_eq!(
                user_gains_cumulative.ray as u64,
                stability_pool_coll_absorbed.ray / 2
            );

            assert_eq!(
                total_gains_cumulative.eth,
                stability_pool_coll_absorbed.eth as u128
            );
            assert_eq!(
                total_gains_cumulative.sol,
                stability_pool_coll_absorbed.sol as u128
            );
            assert_eq!(
                total_gains_cumulative.btc,
                stability_pool_coll_absorbed.btc as u128
            );
            assert_eq!(
                total_gains_cumulative.srm,
                stability_pool_coll_absorbed.srm as u128
            );
            assert_eq!(
                total_gains_cumulative.ftt,
                stability_pool_coll_absorbed.ftt as u128
            );
            assert_eq!(
                total_gains_cumulative.ray,
                stability_pool_coll_absorbed.ray as u128
            );

            assert_fuzzy_eq!((*total_user_deposits as u64), 0, SE);
            assert_fuzzy_eq!((*user_deposits as u64), 0, SE);

            // Product gets reset and epoch incremented
            assert_eq!(stability_pool_state.p, ONE);
            assert_eq!(stability_pool_state.current_epoch, 1);
        }

        // 2. Check borrowing market
        let stability_pool_debt_absored =
            borrow_split.amount_to_borrow * sp_ratio.numerator / sp_ratio.denominator;

        let total_deposited_amount = deposits_lamports.mul_scalar(num_borrowers as u64);
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
        let col_redistributed = deposits_lamports
            .sub(&stability_pool_coll_absorbed)
            .div_scalar((num_borrowers - 1) as u64);

        assert_fuzzy_eq!(
            deposited!(borrowing_users[1], CollateralToken::SOL),
            deposits_lamports.sol + col_redistributed.sol,
            sol_to_lamports(0.01)
        );
        assert_fuzzy_eq!(
            deposited!(borrowing_users[1], CollateralToken::ETH),
            deposits_lamports.eth + col_redistributed.eth,
            sol_to_lamports(0.01)
        );
        assert_fuzzy_eq!(
            deposited!(borrowing_users[1], CollateralToken::BTC),
            deposits_lamports.btc + col_redistributed.btc,
            sol_to_lamports(0.01)
        );
        assert_fuzzy_eq!(
            deposited!(borrowing_users[1], CollateralToken::SRM),
            deposits_lamports.srm + col_redistributed.srm,
            sol_to_lamports(0.01)
        );
        assert_fuzzy_eq!(
            deposited!(borrowing_users[1], CollateralToken::FTT),
            deposits_lamports.ftt + col_redistributed.ftt,
            sol_to_lamports(0.01)
        );
        assert_fuzzy_eq!(
            deposited!(borrowing_users[1], CollateralToken::RAY),
            deposits_lamports.ray + col_redistributed.ray,
            sol_to_lamports(0.01)
        );

        // Market
        assert_eq!(
            deposited!(market, CollateralToken::SOL),
            total_deposited_amount.sol
                - stability_pool_coll_absorbed.sol
                - liquidator_fees.sol
                - clearer_fees.sol
        );
        assert_eq!(
            deposited!(market, CollateralToken::ETH),
            total_deposited_amount.eth
                - stability_pool_coll_absorbed.eth
                - liquidator_fees.eth
                - clearer_fees.eth
        );
        assert_eq!(
            deposited!(market, CollateralToken::BTC),
            total_deposited_amount.btc
                - stability_pool_coll_absorbed.btc
                - liquidator_fees.btc
                - clearer_fees.btc
        );
        assert_eq!(
            deposited!(market, CollateralToken::SRM),
            total_deposited_amount.srm
                - stability_pool_coll_absorbed.srm
                - liquidator_fees.srm
                - clearer_fees.srm
        );
        assert_eq!(
            deposited!(market, CollateralToken::FTT),
            total_deposited_amount.ftt
                - stability_pool_coll_absorbed.ftt
                - liquidator_fees.ftt
                - clearer_fees.ftt
        );
        assert_eq!(
            deposited!(market, CollateralToken::RAY),
            total_deposited_amount.ray
                - stability_pool_coll_absorbed.ray
                - liquidator_fees.ray
                - clearer_fees.ray
        );

        assert_eq!(
            market.stablecoin_borrowed,
            total_borrowed_amount - stability_pool_debt_absored
        );

        println!("Redistrib User after liquidation: {:?}", borrowing_users[1]);
    }
}
