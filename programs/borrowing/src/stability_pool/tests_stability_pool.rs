#![allow(unaligned_references)]
#[cfg(test)]
mod tests {

    // const SE: u64 = USDH::from(0.01);
    const SE: u64 = 10;

    use std::cell::RefCell;

    use crate::{
        assert_fuzzy_eq,
        stability_pool::tests_utils::utils::{
            assert_balances, assert_balances_multicollateral, new_stability_users,
        },
        state::epoch_to_scale_to_sum::EpochToScaleToSum,
        utils::{coretypes::USDH, math::coll_to_lamports},
    };
    use anchor_lang::solana_program::native_token::sol_to_lamports;

    use crate::stability_pool::stability_pool_operations;
    use crate::state::*;
    use crate::CollateralToken::*;

    #[test]
    fn test_stability_stability_pool_simple() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();
        let mut user_two = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_two);

        println!("user_one {:}", user_one.to_state_string());
        println!("user_two {:?}", user_two.to_state_string());

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            sol_to_lamports(100.0),
            now_timestamp,
        )
        .unwrap();

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            sol_to_lamports(100.0),
            now_timestamp,
        )
        .unwrap();

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts::of_token(sol_to_lamports(10.0), SOL),
            USDH::from(50.0),
            now_timestamp,
        )
        .unwrap();

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts::of_token(sol_to_lamports(10.0), SOL),
            USDH::from(50.0),
            now_timestamp,
        )
        .unwrap();

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        // Expecting idempotence
        for _ in 0..10 {
            stability_pool_operations::liquidate(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                CollateralAmounts::of_token(sol_to_lamports(10.0), SOL),
                USDH::from(10.0),
                now_timestamp,
            )
            .unwrap();
        }

        // Expecting idempotence
        for _ in 0..10 {
            utils::harvest_all_liquidation_gains(
                &mut stability_pool_state,
                &mut user_one,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
            );
        }
    }

    #[test]
    fn test_stability_one_user_takes_all() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts::of_token(sol_to_lamports(10.0), SOL),
            USDH::from(10.0),
            now_timestamp,
        )
        .unwrap();

        let total_gains_pending = &stability_pool_state.pending_collateral_gains;
        assert_eq!(total_gains_pending.sol as u64, sol_to_lamports(10.0));

        println!("BH User {}", user_one.to_state_string());
        println!("BH SP {}", stability_pool_state.to_state_string());

        stability_pool_operations::update_pending_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
        )
        .unwrap();

        {
            println!("AH User {}", user_one.to_state_string());
            println!("AH SP {}", stability_pool_state.to_state_string());

            let user_gains_pending = &user_one.pending_gains_per_user;
            let user_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_deposits = &user_one.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;

            assert_eq!(user_gains_pending.sol as u64, sol_to_lamports(10.0));
            assert_eq!(user_gains_cumulative.sol as u64, sol_to_lamports(0.0));
            assert_eq!(total_gains_cumulative.sol as u64, sol_to_lamports(10.0));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(90.0)), SE);

            assert_fuzzy_eq!((*user_deposits as u64), (USDH::from(90.0)), SE);
        }

        for token in [
            StabilityToken::SOL,
            StabilityToken::ETH,
            StabilityToken::BTC,
            StabilityToken::FTT,
            StabilityToken::RAY,
            StabilityToken::SRM,
        ] {
            stability_pool_operations::harvest_pending_gains(
                &mut stability_pool_state,
                &mut user_one,
                token,
            )
            .unwrap();
        }

        {
            let user_gains_pending = &user_one.pending_gains_per_user;
            let user_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_deposits = &user_one.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;

            println!("User {}", user_one.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_eq!(user_gains_pending.sol as u64, sol_to_lamports(0.0));
            assert_eq!(user_gains_cumulative.sol as u64, sol_to_lamports(10.0));
            assert_eq!(total_gains_cumulative.sol as u64, sol_to_lamports(10.0));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(90.0)), SE);

            assert_fuzzy_eq!((*user_deposits as u64), (USDH::from(90.0)), SE);
        }
    }

    #[test]
    fn test_stability_one_user_multi_collateral_takes_all() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts {
                sol: coll_to_lamports(10.0, SOL),
                eth: coll_to_lamports(11.0, ETH),
                btc: coll_to_lamports(12.0, BTC),
                srm: coll_to_lamports(13.0, SRM),
                ray: coll_to_lamports(14.0, RAY),
                ftt: coll_to_lamports(15.0, FTT),
            },
            USDH::from(10.0),
            now_timestamp,
        )
        .unwrap();

        let total_gains_pending = &stability_pool_state.pending_collateral_gains;
        assert_eq!(total_gains_pending.sol as u64, coll_to_lamports(10.0, SOL));
        assert_eq!(total_gains_pending.eth as u64, coll_to_lamports(11.0, ETH));
        assert_eq!(total_gains_pending.btc as u64, coll_to_lamports(12.0, BTC));
        assert_eq!(total_gains_pending.srm as u64, coll_to_lamports(13.0, SRM));
        assert_eq!(total_gains_pending.ray as u64, coll_to_lamports(14.0, RAY));
        assert_eq!(total_gains_pending.ftt as u64, coll_to_lamports(15.0, FTT));

        println!("BH User {}", user_one.to_state_string());
        println!("BH SP {}", stability_pool_state.to_state_string());

        stability_pool_operations::update_pending_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
        )
        .unwrap();

        {
            println!("AH User {}", user_one.to_state_string());
            println!("AH SP {}", stability_pool_state.to_state_string());

            let user_gains_pending = &user_one.pending_gains_per_user;
            let user_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_deposits = &user_one.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;

            assert_eq!(user_gains_pending.sol, coll_to_lamports(10.0, SOL));
            assert_eq!(user_gains_pending.eth, coll_to_lamports(11.0, ETH));
            assert_eq!(user_gains_pending.btc, coll_to_lamports(12.0, BTC));
            assert_eq!(user_gains_pending.srm, coll_to_lamports(13.0, SRM));
            assert_eq!(user_gains_pending.ray, coll_to_lamports(14.0, RAY));
            assert_eq!(user_gains_pending.ftt, coll_to_lamports(15.0, FTT));

            assert_eq!(user_gains_cumulative.sol as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(user_gains_cumulative.eth as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(user_gains_cumulative.btc as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(user_gains_cumulative.srm as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(user_gains_cumulative.ray as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(user_gains_cumulative.ftt as u64, coll_to_lamports(0.0, FTT));

            assert_eq!(
                total_gains_cumulative.sol as u64,
                coll_to_lamports(10.0, SOL)
            );
            assert_eq!(
                total_gains_cumulative.eth as u64,
                coll_to_lamports(11.0, ETH)
            );
            assert_eq!(
                total_gains_cumulative.btc as u64,
                coll_to_lamports(12.0, BTC)
            );
            assert_eq!(
                total_gains_cumulative.srm as u64,
                coll_to_lamports(13.0, SRM)
            );
            assert_eq!(
                total_gains_cumulative.ray as u64,
                coll_to_lamports(14.0, RAY)
            );
            assert_eq!(
                total_gains_cumulative.ftt as u64,
                coll_to_lamports(15.0, FTT)
            );

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(90.0)), SE);

            assert_fuzzy_eq!((*user_deposits as u64), (USDH::from(90.0)), SE);
        }

        for token in [
            StabilityToken::SOL,
            StabilityToken::ETH,
            StabilityToken::BTC,
            StabilityToken::FTT,
            StabilityToken::RAY,
            StabilityToken::SRM,
        ] {
            stability_pool_operations::harvest_pending_gains(
                &mut stability_pool_state,
                &mut user_one,
                token,
            )
            .unwrap();
        }

        {
            let user_gains_pending = &user_one.pending_gains_per_user;
            let user_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_deposits = &user_one.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;

            println!("User {}", user_one.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_eq!(user_gains_pending.sol, coll_to_lamports(0.0, SOL));
            assert_eq!(user_gains_pending.eth, coll_to_lamports(0.0, ETH));
            assert_eq!(user_gains_pending.btc, coll_to_lamports(0.0, BTC));
            assert_eq!(user_gains_pending.srm, coll_to_lamports(0.0, SRM));
            assert_eq!(user_gains_pending.ray, coll_to_lamports(0.0, RAY));
            assert_eq!(user_gains_pending.ftt, coll_to_lamports(0.0, FTT));

            assert_eq!(
                user_gains_cumulative.sol as u64,
                coll_to_lamports(10.0, SOL)
            );
            assert_eq!(
                user_gains_cumulative.eth as u64,
                coll_to_lamports(11.0, ETH)
            );
            assert_eq!(
                user_gains_cumulative.btc as u64,
                coll_to_lamports(12.0, BTC)
            );
            assert_eq!(
                user_gains_cumulative.srm as u64,
                coll_to_lamports(13.0, SRM)
            );
            assert_eq!(
                user_gains_cumulative.ray as u64,
                coll_to_lamports(14.0, RAY)
            );
            assert_eq!(
                user_gains_cumulative.ftt as u64,
                coll_to_lamports(15.0, FTT)
            );

            assert_eq!(
                total_gains_cumulative.sol as u64,
                coll_to_lamports(10.0, SOL)
            );
            assert_eq!(
                total_gains_cumulative.eth as u64,
                coll_to_lamports(11.0, ETH)
            );
            assert_eq!(
                total_gains_cumulative.btc as u64,
                coll_to_lamports(12.0, BTC)
            );
            assert_eq!(
                total_gains_cumulative.srm as u64,
                coll_to_lamports(13.0, SRM)
            );
            assert_eq!(
                total_gains_cumulative.ray as u64,
                coll_to_lamports(14.0, RAY)
            );
            assert_eq!(
                total_gains_cumulative.ftt as u64,
                coll_to_lamports(15.0, FTT)
            );

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(90.0)), SE);

            assert_fuzzy_eq!((*user_deposits as u64), (USDH::from(90.0)), SE);
        }
    }

    #[test]
    fn test_stability_two_users_split() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();
        let mut user_two = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_two);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        // No harvest, no liquidation
        {
            let user_one_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_one_deposits = &user_one.deposited_stablecoin;
            let user_two_gains_cumulative = &user_two.cumulative_gains_per_user;
            let user_two_deposits = &user_two.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;
            let total_gains_pending = &stability_pool_state.pending_collateral_gains;
            assert_eq!(total_gains_pending.sol as u64, sol_to_lamports(0.0));

            println!("User One {}", user_one.to_state_string());
            println!("User Two {}", user_two.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_eq!(user_one_gains_cumulative.sol as u64, sol_to_lamports(0.0));
            assert_eq!(user_two_gains_cumulative.sol as u64, sol_to_lamports(0.0));
            assert_eq!(total_gains_cumulative.sol as u64, sol_to_lamports(0.0));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(200.0)), SE);

            assert_fuzzy_eq!((*user_one_deposits as u64), (USDH::from(100.0)), SE);
            assert_fuzzy_eq!((*user_two_deposits as u64), (USDH::from(100.0)), SE);
        }

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts::of_token(sol_to_lamports(10.0), SOL),
            USDH::from(10.0),
            now_timestamp,
        )
        .unwrap();

        // No harvest, one liquidation
        {
            let user_one_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_one_deposits = &user_one.deposited_stablecoin;
            let user_two_gains_cumulative = &user_two.cumulative_gains_per_user;
            let user_two_deposits = &user_two.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;
            let total_gains_pending = &stability_pool_state.pending_collateral_gains;
            assert_eq!(total_gains_pending.sol as u64, sol_to_lamports(10.0));

            println!("User One {}", user_one.to_state_string());
            println!("User Two {}", user_two.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_eq!(user_one_gains_cumulative.sol as u64, sol_to_lamports(0.0));
            assert_eq!(user_two_gains_cumulative.sol as u64, sol_to_lamports(0.0));
            assert_eq!(total_gains_cumulative.sol as u64, sol_to_lamports(10.0));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(190.0)), SE);

            assert_fuzzy_eq!((*user_one_deposits as u64), (USDH::from(100.0)), SE);
            assert_fuzzy_eq!((*user_two_deposits as u64), (USDH::from(100.0)), SE);
        }

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
                &mut user_one,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
                token,
            )
            .unwrap();
        }

        // One harvest, one liquidation
        {
            let user_one_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_one_deposits = &user_one.deposited_stablecoin;
            let user_two_gains_cumulative = &user_two.cumulative_gains_per_user;
            let user_two_deposits = &user_two.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;
            let total_gains_pending = &stability_pool_state.pending_collateral_gains;
            assert_eq!(total_gains_pending.sol as u64, sol_to_lamports(5.0));

            println!("User One {}", user_one.to_state_string());
            println!("User Two {}", user_two.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_eq!(user_one_gains_cumulative.sol as u64, sol_to_lamports(5.0));
            assert_eq!(user_two_gains_cumulative.sol as u64, sol_to_lamports(0.0));
            assert_eq!(total_gains_cumulative.sol as u64, sol_to_lamports(10.0));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(190.0)), SE);

            assert_fuzzy_eq!((*user_one_deposits as u64), (USDH::from(95.0)), SE);
            assert_fuzzy_eq!((*user_two_deposits as u64), (USDH::from(100.0)), SE);
        }

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        // Two harvest, one liquidation
        {
            let user_one_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_one_deposits = &user_one.deposited_stablecoin;
            let user_two_gains_cumulative = &user_two.cumulative_gains_per_user;
            let user_two_deposits = &user_two.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;
            let total_gains_pending = &stability_pool_state.pending_collateral_gains;
            assert_eq!(total_gains_pending.sol as u64, sol_to_lamports(0.0));

            println!("User One {}", user_one.to_state_string());
            println!("User Two {}", user_two.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_eq!(user_one_gains_cumulative.sol as u64, sol_to_lamports(5.0));
            assert_eq!(user_two_gains_cumulative.sol as u64, sol_to_lamports(5.0));
            assert_eq!(total_gains_cumulative.sol as u64, sol_to_lamports(10.0));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(190.0)), SE);

            assert_fuzzy_eq!((*user_one_deposits as u64), (USDH::from(95.0)), SE);
            assert_fuzzy_eq!((*user_two_deposits as u64), (USDH::from(95.0)), SE);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test_stability_two_users_multi_collateral_split_simple() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0; let now_timestamp=0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts
        );

        let mut user_one = StabilityProviderState::default();
        let mut user_two = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_two);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(120.0),
            now_timestamp
        )
        .unwrap();

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            USDH::from(80.0),
            now_timestamp
        )
        .unwrap();

        // No harvest, no liquidation
        {
            let user_one_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_one_deposits = &user_one.deposited_stablecoin;
            let user_two_gains_cumulative = &user_two.cumulative_gains_per_user;
            let user_two_deposits = &user_two.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;
            let total_gains_pending = &stability_pool_state.pending_collateral_gains;

            assert_eq!(total_gains_pending.sol as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(total_gains_pending.eth as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(total_gains_pending.btc as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(total_gains_pending.srm as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(total_gains_pending.ray as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(total_gains_pending.ftt as u64, coll_to_lamports(0.0, FTT));

            println!("User One {}", user_one.to_state_string());
            println!("User Two {}", user_two.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_eq!(user_one_gains_cumulative.sol  as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(user_one_gains_cumulative.eth  as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(user_one_gains_cumulative.btc  as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(user_one_gains_cumulative.srm  as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(user_one_gains_cumulative.ray  as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(user_one_gains_cumulative.ftt  as u64, coll_to_lamports(0.0, FTT));

            assert_eq!(user_two_gains_cumulative.sol as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(user_two_gains_cumulative.eth as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(user_two_gains_cumulative.btc as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(user_two_gains_cumulative.srm as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(user_two_gains_cumulative.ray as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(user_two_gains_cumulative.ftt as u64, coll_to_lamports(0.0, FTT));

            assert_eq!(total_gains_cumulative.sol as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(total_gains_cumulative.eth as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(total_gains_cumulative.btc as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(total_gains_cumulative.srm as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(total_gains_cumulative.ray as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(total_gains_cumulative.ftt as u64, coll_to_lamports(0.0, FTT));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(200.0)),SE);
            assert_fuzzy_eq!((*user_one_deposits as u64), (USDH::from(120.0)), SE);
            assert_fuzzy_eq!((*user_two_deposits as u64), (USDH::from(80.0)), SE);
        }

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts {
                sol: coll_to_lamports(10.0, SOL),
                eth: coll_to_lamports(11.0, ETH),
                btc: coll_to_lamports(12.0, BTC),
                srm: coll_to_lamports(13.0, SRM),
                ray: coll_to_lamports(14.0, RAY),
                ftt: coll_to_lamports(15.0, FTT),
            },
            USDH::from(10.0),
            now_timestamp
        )
        .unwrap();

        // No harvest, one liquidation
        {
            let user_one_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_one_deposits = &user_one.deposited_stablecoin;
            let user_two_gains_cumulative = &user_two.cumulative_gains_per_user;
            let user_two_deposits = &user_two.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;
            let total_gains_pending = &stability_pool_state.pending_collateral_gains;
            assert_eq!(total_gains_pending.sol as u64, sol_to_lamports(10.0));

            println!("User One {}", user_one.to_state_string());
            println!("User Two {}", user_two.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_eq!(user_one_gains_cumulative.sol  as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(user_one_gains_cumulative.eth  as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(user_one_gains_cumulative.btc  as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(user_one_gains_cumulative.srm  as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(user_one_gains_cumulative.ray  as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(user_one_gains_cumulative.ftt  as u64, coll_to_lamports(0.0, FTT));

            assert_eq!(user_two_gains_cumulative.sol  as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(user_two_gains_cumulative.eth  as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(user_two_gains_cumulative.btc  as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(user_two_gains_cumulative.srm  as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(user_two_gains_cumulative.ray  as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(user_two_gains_cumulative.ftt  as u64, coll_to_lamports(0.0, FTT));

            assert_eq!(total_gains_cumulative.sol as u64, coll_to_lamports(10.0, SOL));
            assert_eq!(total_gains_cumulative.eth as u64, coll_to_lamports(11.0, ETH));
            assert_eq!(total_gains_cumulative.btc as u64, coll_to_lamports(12.0, BTC));
            assert_eq!(total_gains_cumulative.srm as u64, coll_to_lamports(13.0, SRM));
            assert_eq!(total_gains_cumulative.ray as u64, coll_to_lamports(14.0, RAY));
            assert_eq!(total_gains_cumulative.ftt as u64, coll_to_lamports(15.0, FTT));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(190.0)),SE);
            assert_fuzzy_eq!((*user_one_deposits as u64), (USDH::from(120.0)), SE);
            assert_fuzzy_eq!((*user_two_deposits as u64), (USDH::from(80.0)), SE);
        }

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        // One harvest, one liquidation
        {
            let user_one_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_one_deposits = &user_one.deposited_stablecoin;
            let user_two_gains_cumulative = &user_two.cumulative_gains_per_user;
            let user_two_deposits = &user_two.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;
            let total_gains_pending = &stability_pool_state.pending_collateral_gains;

            println!("User One {}", user_one.to_state_string());
            println!("User Two {}", user_two.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            // user 1 gets 60%, user 2 gets 40%
            assert_fuzzy_eq!(user_one_gains_cumulative.sol, coll_to_lamports(10.0 * 0.6, SOL), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.eth, coll_to_lamports(11.0 * 0.6, ETH), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.btc, coll_to_lamports(12.0 * 0.6, BTC), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.srm, coll_to_lamports(13.0 * 0.6, SRM), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.ray, coll_to_lamports(14.0 * 0.6, RAY), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.ftt, coll_to_lamports(15.0 * 0.6, FTT), SE);

            assert_eq!(user_two_gains_cumulative.sol as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(user_two_gains_cumulative.eth as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(user_two_gains_cumulative.btc as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(user_two_gains_cumulative.srm as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(user_two_gains_cumulative.ray as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(user_two_gains_cumulative.ftt as u64, coll_to_lamports(0.0, FTT));

            assert_eq!(total_gains_cumulative.sol as u64, coll_to_lamports(10.0, SOL));
            assert_eq!(total_gains_cumulative.eth as u64, coll_to_lamports(11.0, ETH));
            assert_eq!(total_gains_cumulative.btc as u64, coll_to_lamports(12.0, BTC));
            assert_eq!(total_gains_cumulative.srm as u64, coll_to_lamports(13.0, SRM));
            assert_eq!(total_gains_cumulative.ray as u64, coll_to_lamports(14.0, RAY));
            assert_eq!(total_gains_cumulative.ftt as u64, coll_to_lamports(15.0, FTT));

            assert_eq!(total_gains_pending.sol as u64, coll_to_lamports(10.0 * 0.4, SOL));
            assert_eq!(total_gains_pending.eth as u64, coll_to_lamports(11.0 * 0.4, ETH));
            assert_eq!(total_gains_pending.btc as u64, coll_to_lamports(12.0 * 0.4, BTC));
            assert_eq!(total_gains_pending.srm as u64, coll_to_lamports(13.0 * 0.4, SRM));
            assert_eq!(total_gains_pending.ray as u64, coll_to_lamports(14.0 * 0.4, RAY));
            assert_eq!(total_gains_pending.ftt as u64, coll_to_lamports(15.0 * 0.4, FTT));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(190.0)),SE);
            assert_fuzzy_eq!((*user_one_deposits as u64), (USDH::from(114.0)), SE);
            assert_fuzzy_eq!((*user_two_deposits as u64), (USDH::from(80.0)), SE);
        }

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        // Two harvest, one liquidation
        {
            let user_one_gains_cumulative = &user_one.cumulative_gains_per_user;
            let user_one_deposits = &user_one.deposited_stablecoin;
            let user_two_gains_cumulative = &user_two.cumulative_gains_per_user;
            let user_two_deposits = &user_two.deposited_stablecoin;

            let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
            let total_user_deposits = &stability_pool_state.stablecoin_deposited;
            let total_gains_pending = &stability_pool_state.pending_collateral_gains;

            println!("User One {}", user_one.to_state_string());
            println!("User Two {}", user_two.to_state_string());
            println!("SP {}", stability_pool_state.to_state_string());

            assert_fuzzy_eq!(user_one_gains_cumulative.sol as u64, coll_to_lamports(10.0 * 0.6, SOL), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.eth as u64, coll_to_lamports(11.0 * 0.6, ETH), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.btc as u64, coll_to_lamports(12.0 * 0.6, BTC), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.srm as u64, coll_to_lamports(13.0 * 0.6, SRM), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.ray as u64, coll_to_lamports(14.0 * 0.6, RAY), SE);
            assert_fuzzy_eq!(user_one_gains_cumulative.ftt as u64, coll_to_lamports(15.0 * 0.6, FTT), SE);

            assert_fuzzy_eq!(user_two_gains_cumulative.sol as u64, coll_to_lamports(10.0 * 0.4, SOL), SE);
            assert_fuzzy_eq!(user_two_gains_cumulative.eth as u64, coll_to_lamports(11.0 * 0.4, ETH), SE);
            assert_fuzzy_eq!(user_two_gains_cumulative.btc as u64, coll_to_lamports(12.0 * 0.4, BTC), SE);
            assert_fuzzy_eq!(user_two_gains_cumulative.srm as u64, coll_to_lamports(13.0 * 0.4, SRM), SE);
            assert_fuzzy_eq!(user_two_gains_cumulative.ray as u64, coll_to_lamports(14.0 * 0.4, RAY), SE);
            assert_fuzzy_eq!(user_two_gains_cumulative.ftt as u64, coll_to_lamports(15.0 * 0.4, FTT), SE);

            assert_eq!(total_gains_cumulative.sol as u64, coll_to_lamports(10.0, SOL));
            assert_eq!(total_gains_cumulative.eth as u64, coll_to_lamports(11.0, ETH));
            assert_eq!(total_gains_cumulative.btc as u64, coll_to_lamports(12.0, BTC));
            assert_eq!(total_gains_cumulative.srm as u64, coll_to_lamports(13.0, SRM));
            assert_eq!(total_gains_cumulative.ray as u64, coll_to_lamports(14.0, RAY));
            assert_eq!(total_gains_cumulative.ftt as u64, coll_to_lamports(15.0, FTT));

            assert_eq!(total_gains_pending.sol as u64, coll_to_lamports(0.0, SOL));
            assert_eq!(total_gains_pending.eth as u64, coll_to_lamports(0.0, ETH));
            assert_eq!(total_gains_pending.btc as u64, coll_to_lamports(0.0, BTC));
            assert_eq!(total_gains_pending.srm as u64, coll_to_lamports(0.0, SRM));
            assert_eq!(total_gains_pending.ray as u64, coll_to_lamports(0.0, RAY));
            assert_eq!(total_gains_pending.ftt as u64, coll_to_lamports(0.0, FTT));

            assert_fuzzy_eq!((*total_user_deposits as u64), (USDH::from(190.0)),SE);
            assert_fuzzy_eq!((*user_one_deposits as u64), (USDH::from(114.0)), SE);
            assert_fuzzy_eq!((*user_two_deposits as u64), (USDH::from(76.0)), SE);
        }
    }

    #[test]
    fn test_stability_two_users_multi_collateral_split_abstracted() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();
        let mut user_two = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_two);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(130.0),
            now_timestamp,
        )
        .unwrap();

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            USDH::from(70.0),
            now_timestamp,
        )
        .unwrap();

        // No harvest, no liquidation
        assert_balances_multicollateral(
            vec![&user_one, &user_two],
            &stability_pool_state,
            200.0,
            vec![CollateralAmounts::default(), CollateralAmounts::default()],
            vec![130.0, 70.0],
            CollateralAmounts::default(),
            CollateralAmounts::default(),
            None,
        );

        let gains = CollateralAmounts {
            sol: coll_to_lamports(10.0, SOL),
            eth: coll_to_lamports(11.0, ETH),
            btc: coll_to_lamports(12.0, BTC),
            srm: coll_to_lamports(13.0, SRM),
            ray: coll_to_lamports(14.0, RAY),
            ftt: coll_to_lamports(15.0, FTT),
        };

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            gains,
            USDH::from(10.0),
            now_timestamp,
        )
        .unwrap();

        // No harvest, one liquidation
        assert_balances_multicollateral(
            vec![&user_one, &user_two],
            &stability_pool_state,
            190.0,
            vec![CollateralAmounts::default(), CollateralAmounts::default()],
            vec![130.0, 70.0],
            gains,
            gains,
            None,
        );

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        // One harvest, one liquidation
        assert_balances_multicollateral(
            vec![&user_one, &user_two],
            &stability_pool_state,
            190.0,
            vec![gains.mul_percent(65), CollateralAmounts::default()],
            vec![123.5, 70.0],
            gains.mul_percent(35),
            gains,
            // Some(2),
            None,
        );

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        assert_balances_multicollateral(
            vec![&user_one, &user_two],
            &stability_pool_state,
            190.0,
            vec![gains.mul_percent(65), gains.mul_percent(35)],
            vec![123.5, 66.5],
            CollateralAmounts::default(),
            gains,
            None,
        );
    }

    #[test]
    fn test_stability_single_user_harvest_noop() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        // No harvest, no liquidation
        assert_balances(
            &[user_one.clone()],
            &stability_pool_state,
            100.0,
            vec![0.0],
            vec![100.0],
            0.0,
            0.0,
            None,
        );

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts::of_token(sol_to_lamports(10.0), SOL),
            USDH::from(10.0),
            now_timestamp,
        )
        .unwrap();

        // No harvest, one liquidation
        assert_balances(
            &[user_one.clone()],
            &stability_pool_state,
            90.0,
            vec![0.0],
            vec![100.0],
            10.0,
            10.0,
            None,
        );

        // Harvest in a loop, nothing changes
        for _ in 0..5 {
            utils::harvest_all_liquidation_gains(
                &mut stability_pool_state,
                &mut user_one,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
            );

            assert_balances(
                &[user_one.clone()],
                &stability_pool_state,
                90.0,
                vec![10.0],
                vec![90.0],
                0.0,
                10.0,
                None,
            );
        }
    }

    #[test]
    fn test_stability_provide_withdraw() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        assert_balances(
            &[user_one.clone()],
            &stability_pool_state,
            100.0,
            vec![0.0],
            vec![100.0],
            0.0,
            0.0,
            None,
        );

        stability_pool_operations::withdraw_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        assert_balances(
            &[user_one],
            &stability_pool_state,
            0.0,
            vec![0.0],
            vec![0.0],
            0.0,
            0.0,
            None,
        );
    }

    #[test]
    fn test_stability_single_user_big_numbers() {
        // This test ensures that big number operations don't panic

        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let num_users = 1000;
        let mut users = new_stability_users(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            num_users,
            1000000.0,
        );

        assert_balances(
            &users,
            &stability_pool_state,
            1_000_000_000.0,
            vec![0.0; num_users],
            vec![1000000.0; num_users],
            0.0,
            0.0,
            None,
        );

        for _ in 0..100 {
            stability_pool_operations::liquidate(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                CollateralAmounts::of_token(sol_to_lamports(10000.0), SOL),
                USDH::from(100000.0),
                now_timestamp,
            )
            .unwrap();

            // Harvest in a loop, nothing changes
            for i in 0..5 {
                utils::harvest_all_liquidation_gains(
                    &mut stability_pool_state,
                    &mut users[i],
                    &mut epoch_to_scale_to_sum,
                    &mut liquidations.borrow_mut(),
                    now_timestamp,
                );
            }

            let _users = new_stability_users(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                #[cfg(feature = "stress_test")]
                100000,
                #[cfg(not(feature = "stress_test"))]
                100,
                1000000.0,
            );
        }
    }

    #[test]
    fn test_stability_full_depletion_single() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut user_one = StabilityProviderState::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts::of_token(sol_to_lamports(5.0), SOL),
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        assert_balances(
            &[user_one],
            &stability_pool_state,
            0.0,
            vec![5.0],
            vec![0.0],
            0.0,
            5.0,
            None,
        );
    }

    #[test]
    fn test_stability_liquidate_more_than_possible() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut user_one = StabilityProviderState::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(100.0),
            now_timestamp,
        )
        .unwrap();

        let res = stability_pool_operations::liquidate(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            CollateralAmounts::of_token(sol_to_lamports(5.0), SOL),
            USDH::from(110.0),
            now_timestamp,
        );

        assert!(res.is_err());
    }

    #[test]
    fn test_stability_pending_sum_does_not_go_out_of_bounds() {
        // annotating this test with 'ignore' as it's expensive
        // to run it, execute `cargo test -- --ignored`
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut user_one = StabilityProviderState::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        let amount_initial = 100.0;
        let amount_debt = 40.0;

        let num_liquidations = 100_000;
        let sol_coll_gain = 1000.0;

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(amount_initial),
            now_timestamp,
        )
        .unwrap();

        for _ in 0..num_liquidations {
            stability_pool_operations::provide_stability(
                &mut stability_pool_state,
                &mut user_one,
                &mut epoch_to_scale_to_sum,
                USDH::from(amount_debt),
                now_timestamp,
            )
            .unwrap();

            stability_pool_operations::liquidate(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                CollateralAmounts::of_token(sol_to_lamports(sol_coll_gain), SOL),
                USDH::from(amount_debt),
                now_timestamp,
            )
            .unwrap();
        }

        println!("SP {:}", stability_pool_state.to_state_string());

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        println!("SP {:}", stability_pool_state.to_state_string());
        println!("User {:}", user_one.to_state_string());

        // TODO: fix this
        // assert_balances(
        //     &[user_one],
        //     &stability_pool_state,
        //     amount_initial,
        //     vec![sol_coll_gain * (num_liquidations as f64)],
        //     vec![amount_initial],
        //     // TODO: see how to fix this, but seems extremely tiny
        //     // 0.000071427885715% after 100_000 liquidations
        //     0.0,
        //     sol_coll_gain * (num_liquidations as f64),
        //     None,
        // );
    }

    #[test]
    fn test_stability_pending_sum_does_not_go_out_of_bounds_big_numbers() {
        // the point of this test is that it doesn't panic
        // at big numbers computations - everyone is taking a 4 mil loan
        // and liquidating 10_000_000 sol

        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());

        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        let amount_initial = 10_000_000.0;
        let amount_debt = 4_000_000.0;

        // let num_liquidations = 100_000;
        let num_liquidations = 10_000;
        let sol_coll_gain = 10_000_000.0;

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(amount_initial),
            now_timestamp,
        )
        .unwrap();

        for _ in 0..num_liquidations {
            let _users = new_stability_users(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                10,
                1000000.0,
            );

            stability_pool_operations::liquidate(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                CollateralAmounts::of_token(sol_to_lamports(sol_coll_gain), SOL),
                USDH::from(amount_debt),
                now_timestamp,
            )
            .unwrap();
        }

        println!("SP {:}", stability_pool_state.to_state_string());

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        println!("SP {:}", stability_pool_state.to_state_string());
        println!("User {:}", user_one.to_state_string());
    }

    #[test]
    #[ignore]
    fn test_stability_millions_of_depletions() {
        // annotating this test with 'ignore' as it's expensive
        // to run it, execute `cargo test -- --ignored`
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut user_one = StabilityProviderState::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        let amount_debt = 40.0;

        let num_liquidations = 10_000_000;

        for _ in 0..num_liquidations {
            stability_pool_operations::provide_stability(
                &mut stability_pool_state,
                &mut user_one,
                &mut epoch_to_scale_to_sum,
                USDH::from(amount_debt),
                now_timestamp,
            )
            .unwrap();

            stability_pool_operations::liquidate(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                CollateralAmounts::of_token(sol_to_lamports(5.0), SOL),
                USDH::from(amount_debt),
                now_timestamp,
            )
            .unwrap();
        }

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        println!("SP {:}", stability_pool_state.to_state_string());
        println!("User {:}", user_one.to_state_string());

        assert_balances(
            &[user_one],
            &stability_pool_state,
            0.0,
            vec![5.0 * 10_000_000.0],
            vec![0.0],
            0.0,
            5.0 * 10_000_000.0,
            None,
        );
    }

    #[test]
    fn test_stability_full_depletions_sequential() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

        for (deposit, debt_to_offset, coll_gain) in [
            (100.0, 100.0, 10.0),
            (100.0, 100.0, 5.0),
            (100.0, 40.0, 7.0),
        ] {
            stability_pool_operations::provide_stability(
                &mut stability_pool_state,
                &mut user_one,
                &mut epoch_to_scale_to_sum,
                USDH::from(deposit),
                now_timestamp,
            )
            .unwrap();
            // liquidate once
            stability_pool_operations::liquidate(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                CollateralAmounts::of_token(sol_to_lamports(coll_gain / 2.0), SOL),
                USDH::from(debt_to_offset / 2.0),
                now_timestamp,
            )
            .unwrap();

            // liquidate twice
            stability_pool_operations::liquidate(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                CollateralAmounts::of_token(sol_to_lamports(coll_gain / 2.0), SOL),
                USDH::from(debt_to_offset / 2.0),
                now_timestamp,
            )
            .unwrap();
        }

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        assert_balances(
            &[user_one],
            &stability_pool_state,
            60.0,
            // 10 + 5 + 7 = 22
            vec![22.0],
            vec![60.0],
            0.000000003,
            22.0,
            None,
        );
    }

    #[test]
    fn test_stability_sequential_liquidations_no_depletion() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());

        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        // total = 100.0 * 10 = 1,000.0 usd
        // liquidate 99% of that -> remaining 0.01 * 1000  = 10
        let num_users = 25;
        let stability_per_user = 100.0;
        let total_in_sp = (num_users as f64) * stability_per_user;
        let num_liquidations = 1000;
        let remaining_amount = 10.0;
        let liquidation_amount_per_event =
            (total_in_sp - remaining_amount) / (num_liquidations as f64);
        let coll_gain_per_event = 10.0;
        let coll_gain_per_user =
            (coll_gain_per_event * (num_liquidations as f64)) / (num_users as f64);

        let mut users = new_stability_users(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            num_users,
            stability_per_user,
        );

        for _ in 0..num_liquidations {
            stability_pool_operations::liquidate(
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                CollateralAmounts::of_token(sol_to_lamports(coll_gain_per_event), SOL),
                USDH::from(liquidation_amount_per_event),
                now_timestamp,
            )
            .unwrap();
        }

        for user in users.iter_mut() {
            utils::harvest_all_liquidation_gains(
                &mut stability_pool_state,
                user,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
            );
        }

        println!("Stability Pool {}", stability_pool_state.to_state_string());

        assert_balances(
            &[users[0].clone()],
            &stability_pool_state,
            remaining_amount,
            vec![coll_gain_per_user],
            vec![remaining_amount / (num_users as f64)],
            0.000003944,
            coll_gain_per_event * (num_liquidations as f64),
            Some(178),
        );
    }

    #[test]
    fn test_stability_full_depletions_two_users_sequential() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let mut user_one = StabilityProviderState::default();
        let mut user_two = StabilityProviderState::default();

        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        let liquidations = RefCell::new(LiquidationsQueue::default());

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_two);

        for (deposit_one, deposit_two, debt_to_offset, num_liquidations, coll_gain) in [
            (100.0, 100.0, 200.0, 10, 10.0), // each gets 5 sol (total 10)
            (100.0, 0.0, 100.0, 5, 5.0),     // furst user gets 5 sol
            (0.0, 100.0, 100.0, 2, 7.0),     // second user gets 7 sol
            (40.0, 60.0, 50.0, 3, 7.0),      // first user gets 2.8 sol, second 4.2 sol (total 7)
        ] {
            if deposit_one > 0.0 {
                stability_pool_operations::provide_stability(
                    &mut stability_pool_state,
                    &mut user_one,
                    &mut epoch_to_scale_to_sum,
                    USDH::from(deposit_one),
                    now_timestamp,
                )
                .unwrap();
            }

            if deposit_two > 0.0 {
                stability_pool_operations::provide_stability(
                    &mut stability_pool_state,
                    &mut user_two,
                    &mut epoch_to_scale_to_sum,
                    USDH::from(deposit_two),
                    now_timestamp,
                )
                .unwrap();
            }

            for _ in 0..num_liquidations {
                stability_pool_operations::liquidate(
                    &mut stability_pool_state,
                    &mut epoch_to_scale_to_sum,
                    CollateralAmounts::of_token(
                        sol_to_lamports(coll_gain / num_liquidations as f64),
                        SOL,
                    ),
                    USDH::from(debt_to_offset / num_liquidations as f64),
                    now_timestamp,
                )
                .unwrap();
            }
        }

        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );
        utils::harvest_all_liquidation_gains(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
        );

        println!("Stability Pool {}", stability_pool_state.to_state_string());
        println!("Stability Provider {}", user_one.to_state_string());

        assert_balances(
            &[user_one],
            &stability_pool_state,
            50.0,
            vec![12.8, 16.2],
            vec![20.0, 30.0],
            0.000000000,
            29.0,
            None,
        );
    }

    mod utils {
        use crate::stability_pool::stability_pool_operations;
        use crate::state::epoch_to_scale_to_sum::EpochToScaleToSum;
        use crate::state::{
            LiquidationsQueue, StabilityPoolState, StabilityProviderState, StabilityToken,
        };
        use std::cell::RefMut;

        pub fn harvest_all_liquidation_gains(
            stability_pool_state: &mut StabilityPoolState,
            stability_provider_state: &mut StabilityProviderState,
            epoch_to_scale_to_sum: &mut EpochToScaleToSum,
            liquidations_queue: &mut RefMut<LiquidationsQueue>,
            now_timestamp: u64,
        ) {
            for token in [
                StabilityToken::SOL,
                StabilityToken::ETH,
                StabilityToken::BTC,
                StabilityToken::FTT,
                StabilityToken::RAY,
                StabilityToken::SRM,
            ] {
                stability_pool_operations::harvest_liquidation_gains(
                    stability_pool_state,
                    stability_provider_state,
                    epoch_to_scale_to_sum,
                    liquidations_queue,
                    now_timestamp,
                    token,
                )
                .unwrap();
            }
        }
    }
}
