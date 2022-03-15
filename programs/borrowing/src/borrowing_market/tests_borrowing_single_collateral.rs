#![allow(unaligned_references)]
#[cfg(test)]
mod tests {
    const _SE: u64 = 10;
    use crate::{
        borrowing_market::{
            borrowing_operations,
            borrowing_rate::BorrowSplit,
            tests_utils::utils::new_borrowing_users_with_amounts_and_price,
            types::{BorrowStablecoinEffects, DepositCollateralEffects, WithdrawCollateralEffects},
        },
        deposited,
        utils::coretypes::USDH,
        BorrowError, BorrowingMarketState, CollateralAmounts, CollateralToken, StakingPoolState,
        TokenPrices, UserMetadata,
    };
    use anchor_lang::solana_program::native_token::sol_to_lamports;

    #[test]
    fn test_borrowing_initialize() {
        let mut market = BorrowingMarketState::new();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        assert_eq!(market.num_users, 0);
        assert_eq!(market.base_rate_bps, 0);
        assert_eq!(market.last_fee_event, 0);
    }

    #[test]
    fn test_borrowing_approve_trove() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

        assert_eq!(user.user_id, 0);
        assert_eq!(market.num_users, 1);
    }

    #[test]
    fn test_borrowing_approve_trove_multi() {
        let mut market = BorrowingMarketState::new();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        for i in 0..1000000 {
            let mut user = UserMetadata::default();
            borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

            assert_eq!(user.user_id, i);
            assert_eq!(market.num_users, i + 1);
        }
    }

    #[test]
    fn test_borrowing_deposit_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

        let lamports = sol_to_lamports(10.0);
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut user,
            lamports,
            CollateralToken::SOL,
        )
        .unwrap();

        assert_eq!(user.borrowed_stablecoin, 0);
        assert_eq!(user.inactive_collateral.sol, lamports);
        assert_eq!(market.inactive_collateral.sol, lamports);
    }

    #[test]
    fn test_borrowing_deposit_and_withdraw_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

        let lamports = sol_to_lamports(10.0);
        let DepositCollateralEffects {
            collateral_to_transfer_from_user: deposit,
        } = borrowing_operations::deposit_collateral(
            &mut market,
            &mut user,
            lamports,
            CollateralToken::SOL,
        )
        .unwrap();

        assert_eq!(user.borrowed_stablecoin, 0);
        assert_eq!(user.inactive_collateral.sol, lamports);
        assert_eq!(user.deposited_collateral.sol, 0);
        assert_eq!(market.inactive_collateral.sol, lamports);
        assert_eq!(market.deposited_collateral.sol, 0);
        assert_eq!(deposit.token_amount(CollateralToken::SOL), lamports);

        let WithdrawCollateralEffects {
            collateral_to_transfer_to_user: withdraw,
            close_user_metadata,
        } = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut user,
            lamports,
            CollateralToken::SOL,
            &TokenPrices::new(10.0),
        )
        .unwrap();

        assert_eq!(user.borrowed_stablecoin, 0);
        assert_eq!(user.inactive_collateral.sol, 0);
        assert_eq!(market.inactive_collateral.sol, 0);
        assert_eq!(withdraw.token_amount(CollateralToken::SOL), lamports);
        assert_eq!(close_user_metadata, true);
    }

    #[test]
    fn test_borrowing_deposit_collateral_multi() {
        let mut market = BorrowingMarketState::new();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let lamports = sol_to_lamports(10.0);
        let count = 100;
        for _ in 0..count {
            let mut user = UserMetadata::default();

            borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
            borrowing_operations::deposit_collateral(
                &mut market,
                &mut user,
                lamports,
                CollateralToken::SOL,
            )
            .unwrap();

            assert_eq!(user.borrowed_stablecoin, 0);
            assert_eq!(user.inactive_collateral.sol, lamports);
        }
        assert_eq!(market.inactive_collateral.sol, lamports * (count as u64));
    }

    #[test]
    fn test_borrowing_borrow_stablecoin() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let now_timestamp = 0;
        let lamports = sol_to_lamports(10.0);

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut user,
            lamports,
            CollateralToken::SOL,
        )
        .unwrap();

        let amount_requested = USDH::from(200.0);
        let treasury_fee = USDH::from(0.15);
        let staking_fee = USDH::from(0.85);
        let amount_fee = treasury_fee + staking_fee;
        let total_debt = amount_requested + amount_fee;

        let BorrowStablecoinEffects {
            amount_mint_to_fees_vault,
            amount_mint_to_user,
            amount_mint_to_treasury_vault,
        } = borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool_state,
            amount_requested,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        assert_eq!(market.deposited_collateral.sol, lamports);
        assert_eq!(market.stablecoin_borrowed, total_debt);
        assert_eq!(user.deposited_collateral.sol, lamports);
        assert_eq!(user.borrowed_stablecoin, total_debt);

        assert_eq!(amount_mint_to_user, amount_requested);
        assert_eq!(amount_mint_to_fees_vault, staking_fee);
        assert_eq!(amount_mint_to_treasury_vault, treasury_fee);
    }

    #[test]
    fn test_borrowing_borrow_stablecoin_multi() {
        let mut market = BorrowingMarketState::new();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let lamports = sol_to_lamports(10.0);
        let count = 100;
        let amount_to_borrow = USDH::from(200.0);
        let total_borrowed = USDH::from(201.0);
        let _fees = USDH::from(200.0 * 0.005);

        for i in 0..count {
            let mut user = UserMetadata::default();
            borrowing_operations::approve_trove(&mut market, &mut user).unwrap();

            borrowing_operations::deposit_collateral(
                &mut market,
                &mut user,
                lamports,
                CollateralToken::SOL,
            )
            .unwrap();

            borrowing_operations::borrow_stablecoin(
                &mut market,
                &mut user,
                &mut staking_pool_state,
                amount_to_borrow,
                &TokenPrices::new(40.0),
                now_timestamp,
            )
            .unwrap();

            assert_eq!(market.deposited_collateral.sol, lamports * (i + 1));
            assert_eq!(user.deposited_collateral.sol, lamports);
            assert_eq!(user.borrowed_stablecoin, total_borrowed);
        }

        assert_eq!(market.stablecoin_borrowed, total_borrowed * count);
        assert_eq!(market.deposited_collateral.sol, lamports * (count as u64));
    }

    #[test]
    fn test_borrowing_repay() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let lamports = sol_to_lamports(10.0);
        let amount_to_borrow = USDH::from(200.0);
        let _total_borrowed = USDH::from(201.0);
        let _fees = USDH::from(200.0 * 0.005);

        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut user,
            lamports,
            CollateralToken::SOL,
        )
        .unwrap();

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool_state,
            amount_to_borrow,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        println!("User {:#?}", user);
        println!("Market {:#?}", market);

        let amount_borrowed = user.borrowed_stablecoin;
        borrowing_operations::repay_loan(&mut market, &mut user, amount_borrowed).unwrap();

        println!("User {:#?}", user);
        println!("Market {:#?}", market);

        assert_eq!(market.inactive_collateral.sol, lamports);
        assert_eq!(market.stablecoin_borrowed, 0);
        assert_eq!(user.inactive_collateral.sol, lamports);
        assert_eq!(user.borrowed_stablecoin, 0);
    }

    #[test]
    fn test_borrowing_and_withdraw_max_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let extra_coll = 200.0;
        let withdrawable = sol_to_lamports(14.4725); // (201.0 / 40) * 1.1 = 5.5275 SOL required for 110% coll ratio
        let remaining = sol_to_lamports(5.5275);
        let remaining_global = sol_to_lamports(extra_coll + 5.5275);
        let borrow_per_user = USDH::from(200.0);
        let _borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let mut borrowers = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            2,
            &[borrow_per_user, borrow_per_user],
            &[
                CollateralAmounts::of_token_f64(200.0, CollateralToken::SOL),
                CollateralAmounts::of_token_f64(20.0, CollateralToken::SOL),
            ],
            40.0,
            now_timestamp,
        );

        // 100 * 40  = 4000

        let effects = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut borrowers[1],
            withdrawable,
            CollateralToken::SOL,
            &TokenPrices::new(40.0),
        )
        .unwrap();

        assert_eq!(effects.collateral_to_transfer_to_user.sol, withdrawable);
        assert_eq!(deposited!(market, CollateralToken::SOL), remaining_global);
        assert_eq!(market.stablecoin_borrowed, USDH::from(201.0) * 2);
        assert_eq!(deposited!(borrowers[1], CollateralToken::SOL), remaining);
        assert_eq!(borrowers[1].borrowed_stablecoin, USDH::from(201.0));
    }

    #[test]
    fn test_borrowing_and_withdraw_too_much_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        // deposit 10 * 40.0 -> 400 coll in USDH
        // borrow 200        -> 200 USDH
        // mrc = 1.1 -> min coll 200 * 1.1 = 220
        // min sol in collateral = 220 / 40.0 = 5.5
        // max withdrawable = deposited (10) - min (5.5) = 4.5 sol

        let deposit_lamports = sol_to_lamports(10.0);
        let fail_withdraw_amount = sol_to_lamports(5.0);
        let amount_to_borrow = USDH::from(200.0);
        let _total_borrowed = 2010000;
        let _fees = USDH::from(1.0);

        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut user,
            deposit_lamports,
            CollateralToken::SOL,
        )
        .unwrap();

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool_state,
            amount_to_borrow,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        let err = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut user,
            fail_withdraw_amount,
            CollateralToken::SOL,
            &TokenPrices::new(40.0),
        );

        assert_eq!(err.err(), Some(BorrowError::NotEnoughCollateral.into()));
        assert_eq!(market.deposited_collateral.sol, sol_to_lamports(10.0));
        assert_eq!(market.stablecoin_borrowed, USDH::from(201.0));
        assert_eq!(user.deposited_collateral.sol, sol_to_lamports(10.0));
        assert_eq!(user.borrowed_stablecoin, USDH::from(201.0));
    }

    #[test]
    fn test_borrowing_repay_full_and_withdraw_all_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let lamports = sol_to_lamports(10.0);
        let amount_to_borrow = USDH::from(200.0);
        let total_borrowed = USDH::from(201.0);
        let _fees = USDH::from(200.0 * 0.005);

        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut user,
            lamports,
            CollateralToken::SOL,
        )
        .unwrap();

        borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool_state,
            amount_to_borrow,
            &TokenPrices::new(40.0),
            now_timestamp,
        )
        .unwrap();

        borrowing_operations::repay_loan(&mut market, &mut user, total_borrowed).unwrap();

        let effects = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut user,
            lamports,
            CollateralToken::SOL,
            &TokenPrices::new(40.0),
        )
        .unwrap();

        assert_eq!(effects.collateral_to_transfer_to_user.sol, lamports);
        assert_eq!(deposited!(market, CollateralToken::SOL), 0);
        assert_eq!(market.stablecoin_borrowed, 0);
        assert_eq!(deposited!(user, CollateralToken::SOL), 0);
        assert_eq!(user.borrowed_stablecoin, 0);
    }

    #[test]
    fn test_deposit_and_withdraw_all_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        let now_timestamp = 0;
        let borrow_per_user = USDH::from(400.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);

        let mut borrowers = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            2,
            &[borrow_per_user, USDH::from(200.0)],
            &[
                CollateralAmounts::of_token_f64(400.0, CollateralToken::SOL),
                CollateralAmounts::of_token_f64(40.0, CollateralToken::SOL),
            ],
            40.0,
            now_timestamp,
        );

        let lamports = borrowers[1].deposited_collateral.sol;

        // We borrowed 200 USDH, repaying it back so we can fully withdraw
        borrowing_operations::repay_loan(&mut market, &mut borrowers[1], USDH::from(201.0))
            .unwrap();

        let effects = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut borrowers[1],
            lamports,
            CollateralToken::SOL,
            &TokenPrices::new(40.0),
        )
        .unwrap();

        assert_eq!(effects.collateral_to_transfer_to_user.sol, lamports);
        assert_eq!(
            market.deposited_collateral.sol,
            borrowers[0].deposited_collateral.sol
        );
        assert_eq!(market.stablecoin_borrowed, borrow_split.amount_to_borrow);
        assert_eq!(deposited!(borrowers[1], CollateralToken::SOL), 0);
        assert_eq!(borrowers[1].borrowed_stablecoin, 0);
    }

    #[test]
    #[should_panic]
    fn test_deposit_and_withdraw_too_much_collateral() {
        let mut market = BorrowingMarketState::new();
        let mut user = UserMetadata::default();
        let _staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let lamports = sol_to_lamports(10.0);

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut user,
            lamports,
            CollateralToken::SOL,
        )
        .unwrap();

        let err = borrowing_operations::withdraw_collateral(
            &mut market,
            &mut user,
            lamports + 1,
            CollateralToken::SOL,
            &TokenPrices::new(40.0),
        );

        assert_eq!(err.err(), Some(BorrowError::NotEnoughCollateral.into()));
        // assert_eq!(
        //     deposited!(market, CollateralToken::SOL),
        //     sol_to_lamports(10.0)
        // );
        // assert_eq!(market.stablecoin_borrowed, 0);
        // assert_eq!(
        //     deposited!(user_positions, 0, CollateralToken::SOL),
        //     sol_to_lamports(10.0)
        // );
        // assert_eq!(user.borrowed_stablecoin, 0);
    }
}
