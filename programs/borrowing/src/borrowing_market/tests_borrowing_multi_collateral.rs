#![allow(unaligned_references)]
#[cfg(test)]
mod tests {

    use anchor_lang::solana_program::native_token::sol_to_lamports;

    use crate::{
        assert_fuzzy_eq,
        borrowing_market::{
            borrowing_operations, tests_utils::utils::calculate_max_withdrawable,
            types::BorrowStablecoinEffects,
        },
        deposited,
        utils::{coretypes::USDH, math::coll_to_lamports},
        BorrowingMarketState, CollateralToken,
        CollateralToken::*,
        Price, StakingPoolState, TokenPrices, UserMetadata,
    };

    #[test]
    fn test_borrowing_multi_deposit_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

        let deposits = [
            (coll_to_lamports(10.0, ETH), ETH),
            (coll_to_lamports(5.0, SOL), SOL),
            (coll_to_lamports(7.6, BTC), BTC),
            (coll_to_lamports(8.3, FTT), FTT),
        ];
        for (amount, asset) in deposits {
            borrowing_operations::deposit_collateral(&mut market, &mut user, amount, asset)
                .unwrap();
        }

        let inactive = user.inactive_collateral;
        assert_eq!(user.borrowed_stablecoin, 0);
        assert_eq!(inactive.token_amount(deposits[0].1), deposits[0].0);
        assert_eq!(inactive.token_amount(deposits[1].1), deposits[1].0);
        assert_eq!(inactive.token_amount(deposits[2].1), deposits[2].0);
        assert_eq!(inactive.token_amount(deposits[3].1), deposits[3].0);

        assert_eq!(
            market.inactive_collateral.token_amount(deposits[0].1),
            deposits[0].0
        );
    }

    #[test]
    fn test_borrowing_multi_deposit_collateral_multi() {
        let mut market = BorrowingMarketState::new();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let deposits = [
            (coll_to_lamports(10.0, ETH), ETH),
            (coll_to_lamports(5.0, SOL), SOL),
            (coll_to_lamports(7.6, BTC), BTC),
            (coll_to_lamports(8.3, FTT), FTT),
        ];

        let count = 100;
        for _ in 0..count {
            let mut user = UserMetadata::default();

            borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

            for (amount, asset) in deposits {
                borrowing_operations::deposit_collateral(&mut market, &mut user, amount, asset)
                    .unwrap();
            }
            let inactive = user.inactive_collateral;
            assert_eq!(user.borrowed_stablecoin, 0);
            assert_eq!(inactive.token_amount(deposits[0].1), deposits[0].0);
            assert_eq!(inactive.token_amount(deposits[1].1), deposits[1].0);
            assert_eq!(inactive.token_amount(deposits[2].1), deposits[2].0);
            assert_eq!(inactive.token_amount(deposits[3].1), deposits[3].0);
        }

        let count = count as u64;
        let inactive = market.inactive_collateral;
        assert_eq!(inactive.token_amount(deposits[0].1), deposits[0].0 * count);
        assert_eq!(inactive.token_amount(deposits[1].1), deposits[1].0 * count);
        assert_eq!(inactive.token_amount(deposits[2].1), deposits[2].0 * count);
        assert_eq!(inactive.token_amount(deposits[3].1), deposits[3].0 * count);
    }

    #[test]
    fn test_borrowing_multi_borrow_stablecoin() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let deposits = [
            (coll_to_lamports(20.0, ETH), ETH),
            (coll_to_lamports(10.0, SOL), SOL),
            (coll_to_lamports(15.2, BTC), BTC),
            (coll_to_lamports(16.6, FTT), FTT),
        ];

        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

        for (amount, asset) in deposits {
            borrowing_operations::deposit_collateral(&mut market, &mut user, amount, asset)
                .unwrap();
        }

        let amount_to_borrow = USDH::from(200.0);
        let total_borrowed = USDH::from(201.0);
        let treasury_fee = USDH::from(0.15);
        let staking_fee = USDH::from(0.85);

        let BorrowStablecoinEffects {
            amount_mint_to_fees_vault,
            amount_mint_to_user,
            amount_mint_to_treasury_vault,
        } = borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool_state,
            amount_to_borrow,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        assert_eq!(deposited!(market, deposits[0].1), deposits[0].0);
        assert_eq!(deposited!(market, deposits[1].1), deposits[1].0);
        assert_eq!(deposited!(market, deposits[2].1), deposits[2].0);
        assert_eq!(deposited!(market, deposits[3].1), deposits[3].0);

        assert_eq!(deposited!(user, deposits[0].1), deposits[0].0);
        assert_eq!(deposited!(user, deposits[1].1), deposits[1].0);
        assert_eq!(deposited!(user, deposits[2].1), deposits[2].0);
        assert_eq!(deposited!(user, deposits[3].1), deposits[3].0);

        assert_eq!(market.stablecoin_borrowed, total_borrowed);
        assert_eq!(user.borrowed_stablecoin, total_borrowed);

        assert_eq!(amount_mint_to_user, amount_to_borrow);
        assert_eq!(amount_mint_to_fees_vault, staking_fee);
        assert_eq!(amount_mint_to_treasury_vault, treasury_fee);
    }

    #[test]
    fn test_borrowing_multi_borrow_stablecoin_multi() {
        let mut market = BorrowingMarketState::new();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let deposits = [
            (coll_to_lamports(20.0, ETH), ETH),
            (coll_to_lamports(10.0, SOL), SOL),
            (coll_to_lamports(15.2, BTC), BTC),
            (coll_to_lamports(16.6, FTT), FTT),
        ];

        let count = 100;
        let amount_requested = USDH::from(200.0);
        let amount_fee = USDH::from(1.0);
        let total_debt = amount_requested + amount_fee;

        for i in 0..count {
            let mut user = UserMetadata::default();
            borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

            for (amount, asset) in deposits {
                borrowing_operations::deposit_collateral(&mut market, &mut user, amount, asset)
                    .unwrap();
            }

            borrowing_operations::borrow_stablecoin(
                &mut market,
                &mut user,
                &mut staking_pool_state,
                amount_requested,
                &TokenPrices::new(40.0),
                now_timestamp,
            )
            .unwrap();

            assert_eq!(deposited!(market, deposits[0].1), deposits[0].0 * (i + 1));
            assert_eq!(deposited!(market, deposits[1].1), deposits[1].0 * (i + 1));
            assert_eq!(deposited!(market, deposits[2].1), deposits[2].0 * (i + 1));
            assert_eq!(deposited!(market, deposits[3].1), deposits[3].0 * (i + 1));

            assert_eq!(deposited!(user, deposits[0].1), deposits[0].0);
            assert_eq!(deposited!(user, deposits[1].1), deposits[1].0);
            assert_eq!(deposited!(user, deposits[2].1), deposits[2].0);
            assert_eq!(deposited!(user, deposits[3].1), deposits[3].0);

            assert_eq!(user.borrowed_stablecoin, total_debt);
        }

        let count = count as u64;
        assert_eq!(market.stablecoin_borrowed, total_debt * count);
        assert_eq!(deposited!(market, deposits[0].1), deposits[0].0 * count);
        assert_eq!(deposited!(market, deposits[1].1), deposits[1].0 * count);
        assert_eq!(deposited!(market, deposits[2].1), deposits[2].0 * count);
        assert_eq!(deposited!(market, deposits[3].1), deposits[3].0 * count);
    }

    #[test]
    fn test_borrowing_multi_repay() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let deposits = [
            (coll_to_lamports(20.0, ETH), ETH),
            (coll_to_lamports(10.0, SOL), SOL),
            (coll_to_lamports(15.2, BTC), BTC),
            (coll_to_lamports(16.6, FTT), FTT),
        ];

        let amount_requested = USDH::from(200.0);
        let amount_fee = USDH::from(1.0);
        let total_debt = amount_requested + amount_fee;

        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

        for (amount, asset) in deposits {
            borrowing_operations::deposit_collateral(&mut market, &mut user, amount, asset)
                .unwrap();
        }

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool_state,
            amount_requested,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        borrowing_operations::repay_loan(&mut market, &mut user, total_debt).unwrap();
        let mkt_inac = market.inactive_collateral;
        let usr_inac = user.inactive_collateral;

        assert_eq!(mkt_inac.token_amount(deposits[0].1), deposits[0].0);
        assert_eq!(mkt_inac.token_amount(deposits[1].1), deposits[1].0);
        assert_eq!(mkt_inac.token_amount(deposits[2].1), deposits[2].0);
        assert_eq!(mkt_inac.token_amount(deposits[3].1), deposits[3].0);

        assert_eq!(market.stablecoin_borrowed, 0);

        assert_eq!(usr_inac.token_amount(deposits[0].1), deposits[0].0);
        assert_eq!(usr_inac.token_amount(deposits[1].1), deposits[1].0);
        assert_eq!(usr_inac.token_amount(deposits[2].1), deposits[2].0);
        assert_eq!(usr_inac.token_amount(deposits[3].1), deposits[3].0);

        assert_eq!(user.borrowed_stablecoin, 0);
    }

    #[test]
    fn test_borrowing_multi_and_withdraw_max_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut last_user = UserMetadata::default();
        let mut user = UserMetadata::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let prices = TokenPrices {
            sol: Price::from_f64(40.0, SOL),
            eth: Price::from_f64(20.0, ETH),
            btc: Price::from_f64(10.0, BTC),
            srm: Price::from_f64(30.0, SRM),
            ray: Price::from_f64(15.0, RAY),
            ftt: Price::from_f64(0.22, FTT),
        };

        let deposits = [
            (coll_to_lamports(5.0, SOL), SOL),
            (coll_to_lamports(10.0, ETH), ETH),
            (coll_to_lamports(7.6, BTC), BTC),
            (coll_to_lamports(1.0, SRM), SRM),
            (coll_to_lamports(4.0, RAY), RAY),
            (coll_to_lamports(8.3, FTT), FTT),
        ];

        // deposited in usd value (amount * price)
        // 5.0 * 40.0 + 10.0 * 20.0 + 7.6 * 10.0 + 1.0 * 30.0 + 4.0 * 15.0 + 8.3 * 0.22 = 567.826
        // borrowed: 400.0 * 1.005  = 402.0
        // coll ratio = 567.826 / 402.0 = 1.4125024875621892
        // deposits without sol:
        // 10.0 * 20.0 + 7.6 * 10.0 + 1.0 * 30.0 + 4.0 * 15.0 + 8.3 * 0.22 = 367.826
        // min sol required for coll ratio 110% ->
        // 402.0 * 1.1 - 367.826 = 74.37400000000002 / 40.0 = 1.8593500000000005 sol
        // withdrawable sol: 5.0 - 1.8593500000000005  = 3.1406499999999995

        let withdrawable = sol_to_lamports(3.1406499999999995);
        let remaining_lamports = sol_to_lamports(1.8593500000000005);
        let amount_to_borrow = USDH::from(400.0);
        let total_borrowed = USDH::from(402.0);
        let _fees = USDH::from(2.0);
        let last_user_sol = sol_to_lamports(100.0);
        borrowing_operations::approve_trove(&mut market, &mut last_user).unwrap();
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut last_user,
            last_user_sol,
            CollateralToken::SOL,
        )
        .unwrap();
        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut last_user,
            &mut staking_pool_state,
            amount_to_borrow,
            &prices,
            now_timestamp,
        )
        .unwrap();

        for (amount, asset) in deposits {
            borrowing_operations::deposit_collateral(&mut market, &mut user, amount, asset)
                .unwrap();
        }

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool_state,
            amount_to_borrow,
            &prices,
            now_timestamp,
        )
        .unwrap();

        let effects = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut user,
            withdrawable,
            CollateralToken::SOL,
            &prices,
        )
        .unwrap();

        assert_eq!(effects.collateral_to_transfer_to_user.sol, withdrawable);
        let exp_market_remaining = remaining_lamports + last_user_sol;
        // TODO check to see if rounding error is in the market's favour, not user's
        assert_fuzzy_eq!(market.deposited_collateral.sol, exp_market_remaining, 2); // 1 lamport rounding err
        assert_eq!(market.stablecoin_borrowed, total_borrowed * 2);
        assert_fuzzy_eq!(user.deposited_collateral.sol, remaining_lamports, 2);
        assert_eq!(user.borrowed_stablecoin, total_borrowed);
    }

    #[test]
    #[should_panic]
    fn test_borrowing_multi_and_withdraw_too_much_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let prices = TokenPrices {
            sol: Price::from_f64(40.0, SOL),
            eth: Price::from_f64(20.0, ETH),
            btc: Price::from_f64(10.0, BTC),
            srm: Price::from_f64(30.0, SRM),
            ray: Price::from_f64(15.0, RAY),
            ftt: Price::from_f64(0.22, FTT),
        };

        let deposits = [
            (coll_to_lamports(5.0, SOL), SOL),
            (coll_to_lamports(10.0, ETH), ETH),
            (coll_to_lamports(7.6, BTC), BTC),
            (coll_to_lamports(1.0, SRM), SRM),
            (coll_to_lamports(4.0, RAY), RAY),
            (coll_to_lamports(8.3, FTT), FTT),
        ];

        // deposited in usd value (amount * price)
        // 5.0 * 40.0 + 10.0 * 20.0 + 7.6 * 10.0 + 1.0 * 30.0 + 4.0 * 15.0 + 8.3 * 0.22 = 567.826
        // borrowed: 400.0 * 1.005  = 402.0
        // coll ratio = 567.826 / 402.0 = 1.4125024875621892
        // deposits without sol:
        // 10.0 * 20.0 + 7.6 * 10.0 + 1.0 * 30.0 + 4.0 * 15.0 + 8.3 * 0.22 = 367.826
        // min sol required for coll ratio 110% ->
        // 402.0 * 1.1 - 367.826 = 74.37400000000002 / 40.0 = 1.8593500000000005 sol
        // withdrawable sol: 5.0 - 1.8593500000000005  = 3.1406499999999995

        let withdrawable_lamports = sol_to_lamports(3.1406499999999995);
        let amount_to_borrow = USDH::from(400.0);
        let _total_borrowed = 4020000;
        let _fees = USDH::from(2.0);
        let now_timestamp = 0;

        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

        for (amount, asset) in deposits {
            borrowing_operations::deposit_collateral(&mut market, &mut user, amount, asset)
                .unwrap();
        }

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool_state,
            amount_to_borrow,
            &prices,
            now_timestamp,
        )
        .unwrap();

        let max_withdrawable_pre_liquidation_lamports =
            calculate_max_withdrawable(&prices, &user, CollateralToken::SOL);
        println!(
            "Max withdrawable SOL {}",
            max_withdrawable_pre_liquidation_lamports
        );

        // Try to withdraw too much
        let err = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut user,
            withdrawable_lamports * 2,
            CollateralToken::SOL,
            &prices,
        );
        assert!(err.is_err());
    }
}
