#![allow(unaligned_references)]
#[cfg(test)]
mod tests {

    const SE: u64 = 10;

    use anchor_lang::prelude::Pubkey;
    use anchor_lang::solana_program::native_token::sol_to_lamports;
    use decimal_wad::ratio::Ratio;

    use crate::borrowing_market::borrowing_operations;
    use crate::borrowing_market::borrowing_rate::BorrowSplit;
    use crate::borrowing_market::tests_utils::utils::{
        new_borrowing_users_with_amounts, new_borrowing_users_with_amounts_and_price,
        new_borrowing_users_with_price,
    };
    use crate::borrowing_market::types::{ClearLiquidationGainsEffects, LiquidationEffects};
    use crate::stability_pool::liquidations_queue;
    use crate::stability_pool::stability_pool_operations;
    use crate::stability_pool::tests_liquidations_queue::utils::set_up_market;
    use crate::stability_pool::tests_utils::utils::assert_balances;
    use crate::state::*;
    use crate::utils::consts::{CLEARER_RATE, LIQUIDATOR_RATE};
    use crate::utils::consts::{LIQUIDATIONS_SECONDS_TO_CLAIM_GAINS, ONE};
    use crate::utils::coretypes::USDH;
    use crate::utils::math::coll_to_lamports;
    use crate::{assert_fuzzy_eq, deposited, BorrowError};

    #[test]
    fn test_liquidations_queue_simple() {
        // Liquidate user at 109%, SP takes everything
        // Borrower 2.18 SOL, 200 USDH debt, SOL price is 100.0
        // Stability pool users: 150.0 two users, absorbs everything
        // Liquidate at 109%, expecting SP to absorb it all

        let (
            mut market,
            mut stability_pool_state,
            mut staking_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            _hbb_emissions_start_ts,
            now_timestamp,
            _,
        ) = set_up_market(vec![150.0, 150.0]);

        let borrow_amount = USDH::from(200.0);
        let sol_price = 100.0;

        // Borrowing at a price of 40.0
        let borrow_split = BorrowSplit::from_amount(borrow_amount, market.base_rate_bps);
        let mut borrowers = new_borrowing_users_with_price(
            &mut market,
            &mut staking_pool_state,
            2,
            borrow_amount,
            sol_to_lamports(2.18),
            sol_price + 100.0, // increase the price to allow
            now_timestamp,
        );
        let borrower_collateral = borrowers[0].deposited_collateral;

        assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 0);
        let liquidator = Pubkey::new_unique();
        let LiquidationEffects {
            liquidation_event,
            usd_to_burn_from_stability_pool,
        } = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowers[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(sol_price),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        assert_eq!(
            usd_to_burn_from_stability_pool,
            borrow_split.amount_to_borrow
        );

        assert_eq!(liquidation_event.liquidator, liquidator);
        assert_eq!(
            liquidation_event.collateral_gain_to_liquidator,
            borrower_collateral.mul_bps(40)
        );
        assert_eq!(
            liquidation_event.collateral_gain_to_clearer,
            borrower_collateral.mul_bps(10)
        );
        assert_eq!(
            liquidation_event.collateral_gain_to_stability_pool,
            borrower_collateral.mul_bps(10_000 - 50)
        );

        assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 1);
        let first_liquidation_event = liquidations_queue::get(&mut liquidations.borrow_mut(), 0);
        assert_eq!(liquidation_event, first_liquidation_event);
    }

    #[test]
    fn test_liquidations_queue_multi() {
        // multiple users get liquidated at 109%

        let sol_deposit = 2.18;
        let sol_price = 100.0;
        let borrow_per_user = USDH::from(200.0);
        let num_borrowers = 100;

        let (
            mut market,
            mut stability_pool_state,
            mut staking_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            _hbb_emissions_start_ts,
            now_timestamp,
            _,
        ) = set_up_market(vec![100000000.0, 100000000.0]);

        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let mut borrowers = new_borrowing_users_with_price(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            borrow_per_user,
            sol_to_lamports(sol_deposit),
            sol_price + 100.0,
            now_timestamp,
        );
        let borrower_collateral = borrowers[0].deposited_collateral;

        assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 0);

        for i in 0..(num_borrowers - 1) {
            let liquidator = Pubkey::new_unique();
            let LiquidationEffects {
                liquidation_event,
                usd_to_burn_from_stability_pool,
            } = borrowing_operations::try_liquidate(
                liquidator,
                &mut market,
                &mut borrowers[i],
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                &TokenPrices::new(sol_price),
                &mut liquidations.borrow_mut(),
                now_timestamp,
            )
            .unwrap();

            assert_eq!(
                usd_to_burn_from_stability_pool,
                borrow_split.amount_to_borrow
            );

            assert_eq!(liquidation_event.liquidator, liquidator);
            assert_eq!(
                liquidation_event.collateral_gain_to_liquidator,
                borrower_collateral.mul_bps(40)
            );
            assert_eq!(
                liquidation_event.collateral_gain_to_clearer,
                borrower_collateral.mul_bps(10)
            );
            assert_eq!(
                liquidation_event.collateral_gain_to_stability_pool,
                borrower_collateral.mul_bps(10_000 - 50)
            );

            assert_eq!(
                liquidations_queue::len(&mut liquidations.borrow_mut()),
                i + 1
            );
            let queued_liquidation_event =
                liquidations_queue::get(&mut liquidations.borrow_mut(), i);
            assert_eq!(liquidation_event, queued_liquidation_event);
        }
    }

    #[test]
    fn test_liquidations_queue_cannot_harvest_without_clear_but_can_provide_and_withdraw() {
        // 300 usdh is staked by two SP providers
        // 201.0 usdh is borrowed by 2 borrowers
        // first borrower is liquidated

        // 201.0 * 1.1 = 221.1 / 3 = 73.7
        // deposit 3 sol

        let (
            mut market,
            mut stability_pool_state,
            mut staking_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            _hbb_emissions_start_ts,
            now_timestamp,
            mut sp_providers,
        ) = set_up_market(vec![120.0, 180.0]);

        let borrow_per_user = USDH::from(200.0);
        let sol_deposits = 3.0;
        let liq_price = 73.0;

        let mut sp_one = sp_providers.remove(0);

        assert_balances(
            &[sp_one.clone()],
            &stability_pool_state,
            300.0,
            vec![0.0],
            vec![300.0 * 0.4],
            0.0,
            0.0,
            None,
        );

        let mut borrowers = new_borrowing_users_with_price(
            &mut market,
            &mut staking_pool_state,
            2,
            borrow_per_user,
            sol_to_lamports(sol_deposits),
            liq_price + 100.0,
            now_timestamp,
        );

        let LiquidationEffects {
            liquidation_event: _,
            usd_to_burn_from_stability_pool: _,
        } = borrowing_operations::try_liquidate(
            Pubkey::new_unique(),
            &mut market,
            &mut borrowers[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(liq_price),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        let harvest_effects = stability_pool_operations::harvest_liquidation_gains(
            &mut stability_pool_state,
            &mut sp_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
            StabilityToken::HBB,
        );

        // Cannot harvest because there is a liquidation event queued up
        assert_eq!(
            harvest_effects.err().unwrap(),
            BorrowError::CannotHarvestUntilLiquidationGainsCleared.into()
        );

        stability_pool_operations::update_pending_gains(
            &mut stability_pool_state,
            &mut sp_one,
            &mut epoch_to_scale_to_sum,
        )
        .unwrap();

        // 150 * 0.4  = 60
        // 100.5 * 0.4 = 40.2
        // 60 - 40.2 = 19.799999999999997

        assert_balances(
            &[sp_one.clone()],
            &stability_pool_state,
            300.0 - 201.0,
            vec![0.0],
            vec![(300.0 - 201.0) * 0.4],
            3.0 * 0.995,
            3.0 * 0.995,
            None,
        );

        // However, can provide stability & withdraw stability
        stability_pool_operations::withdraw_stability(
            &mut stability_pool_state,
            &mut sp_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(10.0),
            now_timestamp,
        )
        .unwrap();

        stability_pool_operations::update_pending_gains(
            &mut stability_pool_state,
            &mut sp_one,
            &mut epoch_to_scale_to_sum,
        )
        .unwrap();

        assert_balances(
            &[sp_one.clone()],
            &stability_pool_state,
            300.0 - 201.0 - 10.0,
            vec![0.0],
            vec![(300.0 - 201.0) * 0.4 - 10.0],
            3.0 * 0.995,
            3.0 * 0.995,
            None,
        );

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut sp_one,
            &mut epoch_to_scale_to_sum,
            USDH::from(10.0),
            now_timestamp,
        )
        .unwrap();

        assert_balances(
            &[sp_one.clone()],
            &stability_pool_state,
            300.0 - 201.0,
            vec![0.0],
            vec![(300.0 - 201.0) * 0.4],
            3.0 * 0.995,
            3.0 * 0.995,
            None,
        );

        let harvest_effects = stability_pool_operations::harvest_liquidation_gains(
            &mut stability_pool_state,
            &mut sp_one,
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
            StabilityToken::HBB,
        );

        // Still cannot harvest because there is a liquidation event queued up
        assert_eq!(
            harvest_effects.err().unwrap(),
            BorrowError::CannotHarvestUntilLiquidationGainsCleared.into()
        );
    }

    #[test]
    fn test_liquidations_queue_clear_gains_simple() {
        // This test checks that stability providers cannot claim/harvest gains
        // until the liquidations queue has been cleared

        // 300 usdh is staked by two SP providers
        // 201.0 usdh is borrowed by 2 borrowers
        // collateral is 201.0 * 1.1 = 221.1 / 6 = 36.85 - 1.0 amt per token - so 36.0 price
        // 36.0 * 6 = 216
        // first borrower is liquidated

        let prices = 36.0;
        let borrow_per_user = USDH::from(200.0);
        let coll_amounts = 1.0;
        let num_borrowers = 2;

        let (
            mut market,
            mut stability_pool_state,
            mut staking_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            _hbb_emissions_start_ts,
            now_timestamp,
            mut sp_providers,
        ) = set_up_market(vec![300.0, 300.0]);

        let mut sp_one = sp_providers.remove(0);
        let _sp_two = sp_providers.remove(0);

        let mut borrowers = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            2,
            &vec![borrow_per_user; num_borrowers],
            &[CollateralAmounts {
                sol: coll_to_lamports(coll_amounts, SOL),
                eth: coll_to_lamports(coll_amounts, ETH),
                btc: coll_to_lamports(coll_amounts, BTC),
                srm: coll_to_lamports(coll_amounts, SRM),
                ray: coll_to_lamports(coll_amounts, RAY),
                ftt: coll_to_lamports(coll_amounts, FTT),
            }; 2],
            prices + 100.0, // more such that borrow succeeds
            now_timestamp,
        );

        let liquidator = Pubkey::new_unique();
        let LiquidationEffects {
            liquidation_event: _,
            usd_to_burn_from_stability_pool: _,
        } = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowers[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new_all(prices),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        let clearing_agent = Pubkey::new_unique();
        use CollateralToken::*;

        // assert cannot harvest until pending gains are released

        for token in [SOL, ETH, BTC, FTT, RAY, SRM] {
            let harvest_result = stability_pool_operations::harvest_liquidation_gains(
                &mut stability_pool_state,
                &mut sp_one,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
                StabilityToken::HBB,
            );

            assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 1);
            assert!(harvest_result.is_err());

            let ClearLiquidationGainsEffects {
                clearing_agent_gains,
                stability_pool_gains,
            } = liquidations_queue::clear_liquidation_gains(
                &mut liquidations.borrow_mut(),
                token,
                clearing_agent,
                now_timestamp,
            );

            assert_eq!(
                stability_pool_gains.token_amount(token),
                coll_to_lamports(coll_amounts * 0.995, token)
            );

            assert_eq!(
                clearing_agent_gains.token_amount(token),
                coll_to_lamports(coll_amounts * 0.001, token)
            );

            assert!(!clearing_agent_gains.is_zero());
            assert!(!stability_pool_gains.is_zero());
        }

        for token in [
            StabilityToken::SOL,
            StabilityToken::ETH,
            StabilityToken::BTC,
            StabilityToken::FTT,
            StabilityToken::RAY,
            StabilityToken::SRM,
        ] {
            let harvest_result = stability_pool_operations::harvest_liquidation_gains(
                &mut stability_pool_state,
                &mut sp_one,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
                token,
            );
            // liquidator gains still pending to be received
            assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 1);
            assert!(harvest_result.is_ok());
        }

        for token in [SOL, ETH, BTC, FTT, RAY, SRM] {
            assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 1);
            let ClearLiquidationGainsEffects {
                clearing_agent_gains,
                stability_pool_gains,
            } = liquidations_queue::clear_liquidation_gains(
                &mut liquidations.borrow_mut(),
                token,
                liquidator,
                now_timestamp,
            );

            // liquidator gets their share
            assert_eq!(
                clearing_agent_gains.token_amount(token),
                coll_to_lamports(coll_amounts * 0.004, token)
            );

            assert!(!clearing_agent_gains.is_zero());
            assert!(stability_pool_gains.is_zero());
        }

        for token in [
            StabilityToken::SOL,
            StabilityToken::ETH,
            StabilityToken::BTC,
            StabilityToken::FTT,
            StabilityToken::RAY,
            StabilityToken::SRM,
        ] {
            let harvest_result = stability_pool_operations::harvest_liquidation_gains(
                &mut stability_pool_state,
                &mut sp_one,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
                token,
            );

            // liquidator gains still pending to be received
            assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 0);
            assert!(harvest_result.is_ok());
        }
    }

    #[test]
    fn test_liquidations_queue_clear_gains_batch_many_liquidations() {
        let prices = 18.0;
        let borrow_per_user = USDH::from(200.0);
        let coll_amounts = 2.0;

        let (
            mut market,
            mut stability_pool_state,
            mut staking_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            _hbb_emissions_start_ts,
            now_timestamp,
            mut sp_providers,
        ) = set_up_market(vec![30000000.0, 30000000.0]);

        let mut sp_one = sp_providers.remove(0);

        let _last_borrower = new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            1,
            &vec![borrow_per_user; 2],
            &[CollateralAmounts {
                sol: coll_to_lamports(coll_amounts, SOL),
                eth: coll_to_lamports(coll_amounts, ETH),
                btc: coll_to_lamports(coll_amounts, BTC),
                srm: coll_to_lamports(coll_amounts, SRM),
                ray: coll_to_lamports(coll_amounts, RAY),
                ftt: coll_to_lamports(coll_amounts, FTT),
            }; 2],
            now_timestamp,
        );

        let liquidator = Pubkey::new_unique();
        let clearing_agent = liquidator.clone();

        let num_liquidations = 90;
        for _ in 0..num_liquidations {
            let mut borrowers = new_borrowing_users_with_amounts(
                &mut market,
                &mut staking_pool_state,
                1,
                &vec![borrow_per_user; 2],
                &[CollateralAmounts {
                    sol: coll_to_lamports(coll_amounts, SOL),
                    eth: coll_to_lamports(coll_amounts, ETH),
                    btc: coll_to_lamports(coll_amounts, BTC),
                    srm: coll_to_lamports(coll_amounts, SRM),
                    ray: coll_to_lamports(coll_amounts, RAY),
                    ftt: coll_to_lamports(coll_amounts, FTT),
                }; 2],
                now_timestamp,
            );
            let LiquidationEffects {
                liquidation_event: _,
                usd_to_burn_from_stability_pool: _,
            } = borrowing_operations::try_liquidate(
                liquidator,
                &mut market,
                &mut borrowers[0],
                &mut stability_pool_state,
                &mut epoch_to_scale_to_sum,
                &TokenPrices::new_all(prices),
                &mut liquidations.borrow_mut(),
                now_timestamp,
            )
            .unwrap();
        }

        use CollateralToken::*;

        // assert cannot harvest until pending gains are released
        for token in [SOL, ETH, BTC, FTT, RAY, SRM] {
            let harvest_result = stability_pool_operations::harvest_liquidation_gains(
                &mut stability_pool_state,
                &mut sp_one,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
                StabilityToken::HBB,
            );

            assert_eq!(
                liquidations_queue::len(&mut liquidations.borrow_mut()),
                num_liquidations
            );
            assert!(harvest_result.is_err());

            let ClearLiquidationGainsEffects {
                clearing_agent_gains,
                stability_pool_gains,
            } = liquidations_queue::clear_liquidation_gains(
                &mut liquidations.borrow_mut(),
                token,
                clearing_agent,
                now_timestamp,
            );

            // clearing agent is also liquidator
            assert_eq!(
                clearing_agent_gains.token_amount(token),
                coll_to_lamports((num_liquidations as f64) * coll_amounts * 0.005, token)
            );

            assert_eq!(
                stability_pool_gains.token_amount(token),
                coll_to_lamports((num_liquidations as f64) * coll_amounts * 0.995, token)
            );
        }

        for token in [
            StabilityToken::SOL,
            StabilityToken::ETH,
            StabilityToken::BTC,
            StabilityToken::FTT,
            StabilityToken::RAY,
            StabilityToken::SRM,
        ] {
            let harvest_result = stability_pool_operations::harvest_liquidation_gains(
                &mut stability_pool_state,
                &mut sp_one,
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
                token,
            );
            // liquidator gains still pending to be received
            assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 0);
            assert!(harvest_result.is_ok());
        }
    }

    #[test]
    fn test_liquidations_split_between_stability_pool_and_redistribution() {
        // The stability pool should not be able to cover the debt of the
        // liquidated user, We liquidate 100 USD, we cover 20 usd (2 users) from
        // the stability pool and the rest gets redistributed
        let (
            mut market,
            mut stability_pool_state,
            mut staking_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            _hbb_emissions_start_ts,
            now_timestamp,
            mut sp_providers,
        ) = set_up_market(vec![1000.0, 1000.0]);

        let deposits_lamports = CollateralAmounts {
            sol: coll_to_lamports(15.0, CollateralToken::SOL),
            eth: coll_to_lamports(10.0, CollateralToken::ETH),
            btc: coll_to_lamports(7.6, CollateralToken::BTC),
            ftt: coll_to_lamports(8.3, CollateralToken::FTT),
            ..Default::default()
        };

        // we liquidate at 109%, therefore
        // mv = 15 + 10 + 7.6 + 8.3 = 40.900000000000006 * 100 / 1.1 = 3718.181818181819 / 1.005 = 3699.68340117594
        // therefore debt is 3700.0 * 1.005 = 4090.0 / 3718.5 = 1.099905876025279

        let liquidation_prices = 100.0;
        let borrow_per_user = USDH::from(3700.0);
        let borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);

        let num_borrowers = 10;
        let mut borrowing_users = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            num_borrowers,
            &vec![borrow_per_user; num_borrowers],
            &vec![deposits_lamports; num_borrowers],
            liquidation_prices + 100.0,
            now_timestamp,
        );

        println!("User before liquidation: {:?}", borrowing_users[0]);
        println!("BM before liq {}", market.to_state_string());
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
        let liq_fees = deposits_lamports.mul_bps(LIQUIDATOR_RATE);
        let clr_fees = deposits_lamports.mul_bps(CLEARER_RATE);
        let coll_gain_to_liquidator = effects.liquidation_event.collateral_gain_to_liquidator;

        assert_eq!(liq_fees.sol, coll_gain_to_liquidator.sol);
        assert_eq!(liq_fees.eth, coll_gain_to_liquidator.eth);
        assert_eq!(liq_fees.btc, coll_gain_to_liquidator.btc);
        assert_eq!(liq_fees.srm, coll_gain_to_liquidator.srm);
        assert_eq!(liq_fees.ftt, coll_gain_to_liquidator.ftt);
        assert_eq!(liq_fees.ray, coll_gain_to_liquidator.ray);

        println!("After liq {}", market.to_state_string());
        println!("After liq {}", stability_pool_state.to_state_string());

        // Before liquidation
        // User debt: 37185000
        // User collateral: 409000000
        // Stability pool: 20000000

        // Liquidator + Clearer fee: 0.005 * 409000000 = 2045000

        // Remaining collateral = 409000000 - 2045000 = 406955000
        // SP takes 20000000 / 37185000 = 0.5378512841199409 (pct) of the collateral and debt
        // SP takes 0.5378512841199409 * 37185000 = 20000000 debt
        // SP takes 0.5378512841199409 * 409000000 = 219981175.20505583 coll
        // also there is a new epoch

        // Stability pool users have 0 balance, but a pending gain of
        // 219981175.20505583 / 2 = 109990587.60252792 = 2985000000 each

        // Borrowing market users get redistributed (1 - 0.5378512841199409 = 0.4621487158800591) of the debt & coll
        // 0.4621487158800591 * 37185000 = 17185000 debt
        // 0.4621487158800591 * 406955000 = 188073730.67096946
        // each user gets an extra
        // 462148.7158800591 / 9 = 51349.857320006566 debt
        // 188073730.67096946 / 9 = 20897081.18566327 collateral

        // num active users decreases

        println!("SP {}", sp_providers[0].to_state_string());

        // 1. Check stability pool
        let harvest_result = stability_pool_operations::harvest_liquidation_gains(
            &mut stability_pool_state,
            &mut sp_providers[0],
            &mut epoch_to_scale_to_sum,
            &mut liquidations.borrow_mut(),
            now_timestamp,
            StabilityToken::HBB,
        );

        let clearing_agent = liquidator;
        use CollateralToken::*;

        assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 1);
        assert!(harvest_result.is_err());

        let liquidation_event = liquidations_queue::get(&mut liquidations.borrow_mut(), 0);
        println!("Liquidations event {:?}", liquidation_event);

        let remaining_deposited_coll = deposits_lamports.sub(&liq_fees).sub(&clr_fees);
        let sp_usd_deposits = USDH::from(2000.0);
        let sp_ratio = Ratio::new(sp_usd_deposits, borrow_split.amount_to_borrow);
        let stability_pool_coll_absorbed =
            remaining_deposited_coll.mul_fraction(sp_ratio.numerator, sp_ratio.denominator);

        for token in [SOL, ETH, BTC, FTT] {
            let harvest_result = stability_pool_operations::harvest_liquidation_gains(
                &mut stability_pool_state,
                &mut sp_providers[0],
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
                StabilityToken::HBB,
            );

            assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 1);
            assert!(harvest_result.is_err());

            let ClearLiquidationGainsEffects {
                clearing_agent_gains,
                stability_pool_gains,
            } = liquidations_queue::clear_liquidation_gains(
                &mut liquidations.borrow_mut(),
                token,
                clearing_agent,
                now_timestamp,
            );

            assert_eq!(
                stability_pool_gains.token_amount(token),
                stability_pool_coll_absorbed.token_amount(token)
            );

            assert_eq!(
                clearing_agent_gains.token_amount(token),
                clr_fees.token_amount(token) + liq_fees.token_amount(token)
            );

            assert!(!clearing_agent_gains.is_zero());
            assert!(!stability_pool_gains.is_zero());
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
                &mut sp_providers[0],
                &mut epoch_to_scale_to_sum,
                &mut liquidations.borrow_mut(),
                now_timestamp,
                token,
            )
            .unwrap();
        }

        {
            println!("SP {}", sp_providers[0].to_state_string());

            let user_gains_pending = &sp_providers[0].pending_gains_per_user;
            let user_gains_cumulative = &sp_providers[0].cumulative_gains_per_user;
            let user_deposits = &sp_providers[0].deposited_stablecoin;

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
            borrowing_operations::refresh_positions(&mut &mut market, _user).unwrap();
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
                - liq_fees.sol
                - clr_fees.sol
        );
        assert_eq!(
            deposited!(market, CollateralToken::ETH),
            total_deposited_amount.eth
                - stability_pool_coll_absorbed.eth
                - liq_fees.eth
                - clr_fees.eth
        );
        assert_eq!(
            deposited!(market, CollateralToken::BTC),
            total_deposited_amount.btc
                - stability_pool_coll_absorbed.btc
                - liq_fees.btc
                - clr_fees.btc
        );
        assert_eq!(
            deposited!(market, CollateralToken::SRM),
            total_deposited_amount.srm
                - stability_pool_coll_absorbed.srm
                - liq_fees.srm
                - clr_fees.srm
        );
        assert_eq!(
            deposited!(market, CollateralToken::FTT),
            total_deposited_amount.ftt
                - stability_pool_coll_absorbed.ftt
                - liq_fees.ftt
                - clr_fees.ftt
        );
        assert_eq!(
            deposited!(market, CollateralToken::RAY),
            total_deposited_amount.ray
                - stability_pool_coll_absorbed.ray
                - liq_fees.ray
                - clr_fees.ray
        );

        assert_eq!(
            market.stablecoin_borrowed,
            total_borrowed_amount - stability_pool_debt_absored
        );

        println!("Redistrib User after liquidation: {:?}", borrowing_users[1]);
    }

    #[test]
    fn test_liquidations_queue_clearer_gets_all() {
        let (
            mut market,
            mut stability_pool_state,
            mut staking_pool_state,
            mut epoch_to_scale_to_sum,
            liquidations,
            _hbb_emissions_start_ts,
            now_timestamp,
            _,
        ) = set_up_market(vec![150.0, 150.0]);

        // First borrow needs to be substantial enough such
        // that the system doesn't get straight into recovery mode
        // SOL price is 111.00
        // Borrowing
        // user 2: 200.0 USDH vs 20 SOL -> CR 2201.0 / 200 = 11.005
        // user 1: 200.0 USDH vs 2 SOL  -> CR 222.0 / 201.0 = 1.1044776119402986
        let borrow_per_user = USDH::from(200.0);
        let _borrow_split = BorrowSplit::from_amount(borrow_per_user, market.base_rate_bps);
        let mut borrowers = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            2,
            &[borrow_per_user, borrow_per_user],
            &[
                CollateralAmounts::of_token_f64(20.0, CollateralToken::SOL),
                CollateralAmounts::of_token_f64(2.0, CollateralToken::SOL),
            ],
            111.0, // 110.5% CR
            now_timestamp,
        );

        assert_eq!(liquidations_queue::len(&mut liquidations.borrow_mut()), 0);
        let liquidator = Pubkey::new_unique();
        let _clearing_agent = Pubkey::new_unique();
        let LiquidationEffects { .. } = borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowers[1],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(1.0),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

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

        // still some left for the liquidator
        // still pending
        let liquiation_event = liquidations.borrow().events[0];
        assert_eq!(liquiation_event.collateral_gain_to_clearer.sol, 0);
        assert_ne!(liquiation_event.collateral_gain_to_liquidator.sol, 0);
        assert_eq!(liquiation_event.status, 1);

        // now after 5 seconds, liquidator doesn't do anything
        // so clearer gets everything
        {
            use CollateralToken::*;
            let clearing_agent = Pubkey::new_unique();
            for token in [SOL, ETH, BTC, FTT, RAY, SRM] {
                liquidations_queue::clear_liquidation_gains(
                    &mut liquidations.borrow_mut(),
                    token,
                    clearing_agent,
                    now_timestamp + LIQUIDATIONS_SECONDS_TO_CLAIM_GAINS + 1,
                );
            }
        }

        let liquiation_event = liquidations.borrow().events[0];

        // clearer gets the rest
        // inactive
        assert_eq!(liquiation_event.collateral_gain_to_clearer.sol, 0);
        assert_eq!(liquiation_event.collateral_gain_to_liquidator.sol, 0);
        assert_eq!(liquiation_event.status, 0);
    }
}

#[cfg(test)]
mod utils {
    use std::cell::RefCell;

    use crate::{
        borrowing_market::borrowing_operations,
        stability_pool::stability_pool_operations,
        state::{
            epoch_to_scale_to_sum::EpochToScaleToSum, StabilityPoolState, StabilityProviderState,
        },
        utils::coretypes::USDH,
        BorrowingMarketState, LiquidationsQueue, StakingPoolState,
    };

    pub fn set_up_market(
        stability_provider_amounts_usdh: Vec<f64>,
    ) -> (
        BorrowingMarketState,
        StabilityPoolState,
        StakingPoolState,
        EpochToScaleToSum,
        RefCell<LiquidationsQueue>,
        u64,
        u64,
        Vec<StabilityProviderState>,
    ) {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let hbb_emissions_start_ts = 0;
        let now_timestamp = 0;

        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let sp_providers = create_stability_providers(
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            stability_provider_amounts_usdh,
        );

        (
            market,
            stability_pool_state,
            staking_pool_state,
            epoch_to_scale_to_sum,
            liquidations,
            hbb_emissions_start_ts,
            now_timestamp,
            sp_providers,
        )
    }

    pub fn create_stability_providers(
        stability_pool_state: &mut StabilityPoolState,
        epoch_to_scale_to_sum: &mut EpochToScaleToSum,
        amounts_usdh: Vec<f64>,
    ) -> Vec<StabilityProviderState> {
        amounts_usdh
            .iter()
            .map(|amt| {
                let mut stability_provider = StabilityProviderState::default();
                stability_pool_operations::approve_new_user(
                    stability_pool_state,
                    &mut stability_provider,
                );
                let now_timestamp = 0;

                stability_pool_operations::provide_stability(
                    stability_pool_state,
                    &mut stability_provider,
                    epoch_to_scale_to_sum,
                    USDH::from(*amt),
                    now_timestamp,
                )
                .unwrap();
                stability_provider
            })
            .collect()
    }
}
