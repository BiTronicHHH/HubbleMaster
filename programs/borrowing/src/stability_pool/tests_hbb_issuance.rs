#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use solana_sdk::{clock::SECONDS_PER_DAY, native_token::sol_to_lamports};

    use crate::{
        assert_fuzzy_eq,
        stability_pool::stability_pool_operations::{
            self, issuance_logic::expected_issuance_since_start,
        },
        state::epoch_to_scale_to_sum::EpochToScaleToSum,
        utils::consts::{
            DECIMAL_PRECISION, HBB_FACTOR, SECONDS_PER_YEAR, TOTAL_HBB_TO_STABILITY_POOL,
        },
        LiquidationsQueue, StabilityPoolState, StabilityProviderState,
    };

    const HALF: f64 = 0.5;

    #[test]
    fn test_hbb_issuance_fraction() {
        let total_to_be_issued = 31_000_000.0;

        let one_day = 1.0 / 365.0;
        let two_days = 2.0 / 365.0;
        let one_week = 7.0 / 365.0;
        let two_weeks = 14.0 / 365.0;

        for years in [
            one_day, two_days, one_week, two_weeks, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0,
            10.0,
        ] {
            // expected
            let factor: f64 = 1.0 - HALF.powf(years);
            let issue_so_far = total_to_be_issued * factor;
            let pct_of_total = issue_so_far / total_to_be_issued * 100.0;

            // actual
            let act = expected_issuance_since_start(0, (years * (SECONDS_PER_YEAR as f64)) as u64);
            println!(
                "Issuing after years {:.2} - {:.2} {:.2} HBB Pct {:.2}%",
                years, issue_so_far, act, pct_of_total
            );
        }
    }

    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn test_hbb_issuance_prop(start: u64, now: u64) -> bool {
        if start > now {
            true
        } else {
            let res = expected_issuance_since_start(start, now);
            res <= TOTAL_HBB_TO_STABILITY_POOL * (HBB_FACTOR as u64)
        }
    }

    #[test]
    fn test_hbb_issuance_noop() {
        let res = expected_issuance_since_start(0, 0);
        assert_eq!(res, 0);
    }

    #[test]
    #[should_panic]
    fn test_hbb_issuance_bad_input() {
        expected_issuance_since_start(100, 0);
    }

    #[test]
    fn test_hbb_issuance_decimal() {
        use decimal_wad::{
            common::{TryDiv, TrySub},
            rate::Rate,
        };
        let one = Rate::one();
        let half = one.try_div(2).unwrap();
        // let exp = half.try_pow(0.5).unwrap();

        /* The issuance factor F determines the curvature of the issuance curve.
         *
         * Minutes in one year: 60*24*365 = 525600
         *
         * For 50% of remaining tokens issued each year, with minutes as time units, we have:
         *
         * F ** 525600 = 0.5
         *
         * Re-arranging:
         *
         * 525600 * ln(F) = ln(0.5)
         * F = 0.5 ** (1/525600)
         * F = 0.999998681227695000
         *      1000000000000000000
         */
        const ISSUANCE_FACTOR: u64 = 999998681227695000;

        let seconds_in_one_minute = 60;
        let seconds_passed = SECONDS_PER_YEAR as u64;
        let minutes_pased = seconds_passed / seconds_in_one_minute;

        println!("{:?}", one);
        println!("{:?}", half);

        let factor = Rate::from_scaled_val(ISSUANCE_FACTOR);
        let decimal_precision = one;

        let power = factor.try_pow(minutes_pased).unwrap();

        // half for a year
        let fraction = decimal_precision.try_sub(power).unwrap();
        println!("fraction {}", fraction);

        // that is 10000 / 1000000000000000000
        assert_fuzzy_eq!(
            fraction.to_scaled_val(),
            half.to_scaled_val(),
            Rate::from_scaled_val(10000).to_scaled_val(),
            i128
        );
    }

    #[test]
    fn test_hbb_issuance_day_years() {
        // 1st jan 2021
        let start_time = 1609459200;
        let one_day = start_time + SECONDS_PER_DAY * 1;
        let two_days = start_time + SECONDS_PER_DAY * 2;
        let three_days = start_time + SECONDS_PER_DAY * 3;

        let one_year = start_time + SECONDS_PER_YEAR as u64;
        let two_years = start_time + (SECONDS_PER_YEAR * 2) as u64;

        let after_one_day = expected_issuance_since_start(start_time, one_day);
        let after_two_days = expected_issuance_since_start(start_time, two_days);
        let after_three_days = expected_issuance_since_start(start_time, three_days);

        let after_one_year = expected_issuance_since_start(start_time, one_year);
        let after_two_years = expected_issuance_since_start(start_time, two_years);

        assert_eq!(after_one_day as f64, 58814171800.0);
        assert_eq!(after_two_days as f64, 117516759510.0);
        assert_eq!(after_three_days as f64, 176107974831.0);

        // 588141718 / 10000 = 58,814.1718
        // 31_000_000.0
        // 58814 / 31000000 = 0.001897225806451613 = 0.1% of the supply on day one

        // 1761079748 - 1175167595 = 585912153 =
        // 1175167595 - 588141718 = 587025877
        //
        let day_one_to_day_two = after_two_days - after_one_day;
        let day_two_to_day_three = after_three_days - after_two_days;

        println!("{}", after_one_day);
        println!(
            "{} {} {} ",
            after_two_days,
            after_two_days - after_one_day,
            day_one_to_day_two
        );
        println!(
            "{} {} {} ",
            after_three_days,
            after_three_days - after_two_days,
            day_two_to_day_three
        );

        assert_eq!(day_one_to_day_two as f64, 58702587710.0);
        assert_eq!(day_two_to_day_three as f64, 58591215321.0);

        // halving yearly
        // the number and decimals are so large
        // that this diff doesn't matter
        // left: `15500000000000168.0`,
        // right: `15500000000000000.0`'
        assert_fuzzy_eq!(
            after_one_year,
            TOTAL_HBB_TO_STABILITY_POOL * HBB_FACTOR / 2,
            200
        );
        // left: `23250000000000170.0`,
        // right: `23250000000000000.0`'
        assert_fuzzy_eq!(
            after_two_years,
            TOTAL_HBB_TO_STABILITY_POOL * HBB_FACTOR * 3 / 4,
            200
        );
    }

    // Tests:
    // - [x] add prop based test, it's never < 0 and always < max amount
    // - [x] test one user one year, two years
    // - [ ] test two users three years (maybe)
    // - one user deposits once a day for a year, it adds up to a total of one year

    #[test]
    fn test_hbb_issuance_three_users() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;

        let day_one = SECONDS_PER_DAY * 1;
        let day_two = SECONDS_PER_DAY * 2;
        let day_three = SECONDS_PER_DAY * 3;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();
        let mut user_two = StabilityProviderState::default();

        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_two);

        println!("user_one {}", user_one.to_state_string());
        println!("user_two {}", user_two.to_state_string());

        // This should trigger no issuance, there is nothing there yet
        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            sol_to_lamports(100.0),
            day_one,
        )
        .unwrap();

        // This should trigger first issuance
        // as there is only one user in the pool, he gets the full hbb issuance of first 2 DAYS of
        // 1175167595
        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            sol_to_lamports(100.0),
            day_two,
        )
        .unwrap();

        stability_pool_operations::update_pending_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
        )
        .unwrap();
        let first_user_hbb_gains = user_one.pending_gains_per_user.hbb;
        let total_hbb_issuance = stability_pool_state.pending_collateral_gains.hbb;
        println!("User 1 {}", user_one.to_state_string());
        println!("User 1 {}", first_user_hbb_gains);
        println!("Total {}", total_hbb_issuance);

        // This should trigger second issuance
        // as there are two users with 100 each in the pool, before this deposit
        // both users get the diff between day 3 and day 2 issuance
        // 1761079748 - 1175167595 = 585912153
        // 585912153 / 2 = 292956076.5
        // 292956076.5 / 10000 = 29295.60765
        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
            sol_to_lamports(100.0),
            day_three,
        )
        .unwrap();

        stability_pool_operations::update_pending_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
        )
        .unwrap();

        stability_pool_operations::update_pending_gains(
            &mut stability_pool_state,
            &mut user_two,
            &mut epoch_to_scale_to_sum,
        )
        .unwrap();

        println!("User 1 {}", user_one.to_state_string());
        println!("User 2 {}", user_two.to_state_string());

        let first_user_hbb_gains = user_one.pending_gains_per_user.hbb;
        let second_user_hbb_gains = user_two.pending_gains_per_user.hbb;
        let total_hbb_issuance = stability_pool_state.pending_collateral_gains.hbb;

        // TODO: add way more tests for hbb issuance

        // after one day 588141718
        // after three days 1761079748, diff is 585912153
        // 1761079748 - 1175167595 = 585912153  / 2 = 292956076.5
        // both get 292956076

        println!("User 1 {}", first_user_hbb_gains);
        println!("User 2 {}", second_user_hbb_gains);
        println!("Total {}", total_hbb_issuance);

        // assert_eq!(after_one_day as f64, 58814171800567.0);
        // assert_eq!(after_two_days as f64, 117516759510663.0);
        // assert_eq!(after_three_days as f64, 176107974831124.0);

        // 117516759510663 - 58814171800567 = 58702587710096 / 2 = 29351293855048
        // 176107974831124 - 117516759510663 = 58591215320461 / 2  = 29295607660230.5

        assert_eq!(first_user_hbb_gains, 117516759510 + 29295607660);
        assert_eq!(second_user_hbb_gains, 29295607660);
        assert_eq!(total_hbb_issuance, 176107974831);
    }

    // HBB ISSUANCE TESTS

    #[test]
    fn test_hbb_issuance_one_user_simple() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;

        let day_one = SECONDS_PER_DAY * 1;
        let day_two = SECONDS_PER_DAY * 2;
        let day_three = SECONDS_PER_DAY * 3;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);
        println!("user_one {}", user_one.to_state_string());

        // This should trigger no issuance, there is nothing there yet
        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            sol_to_lamports(100.0),
            day_one,
        )
        .unwrap();

        // This should trigger first issuance
        // as there is only one user in the pool, he gets the full hbb issuance of first 2 DAYS of
        // 1175167595
        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            sol_to_lamports(0.000000001),
            day_two,
        )
        .unwrap();

        stability_pool_operations::provide_stability(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
            sol_to_lamports(0.000000001),
            day_three,
        )
        .unwrap();

        let first_user_hbb_gains = user_one.pending_gains_per_user.hbb;
        let total_hbb_issuance = stability_pool_state.pending_collateral_gains.hbb;

        // after one day 588141718
        // after two days 1175167595
        // after three days 1761079748

        println!("User 1 {}", first_user_hbb_gains);
        println!("Total {}", total_hbb_issuance);

        stability_pool_operations::update_pending_gains(
            &mut stability_pool_state,
            &mut user_one,
            &mut epoch_to_scale_to_sum,
        )
        .unwrap();

        // assert_eq!(first_user_hbb_gains, 1761079748);
        // should have been 1761079747.9.. but given precision loss
        // it's actually rounded down
        assert_eq!(first_user_hbb_gains, 176107974830);
    }

    #[test]
    fn test_hbb_issuance_one_user_daily() {
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let hbb_emissions_start_ts = 0;

        stability_pool_operations::initialize_stability_pool(
            &mut stability_pool_state,
            &mut liquidations.borrow_mut(),
            hbb_emissions_start_ts,
        );

        let mut user_one = StabilityProviderState::default();
        stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);
        println!("user_one {}", user_one.to_state_string());

        // 10 times a day for one year
        let times_per_day = 100;
        for i in 1..366 {
            for j in 1..(times_per_day + 1) {
                stability_pool_operations::provide_stability(
                    &mut stability_pool_state,
                    &mut user_one,
                    &mut epoch_to_scale_to_sum,
                    sol_to_lamports(100.0),
                    SECONDS_PER_DAY * (i - 1) + (SECONDS_PER_DAY / times_per_day * j),
                )
                .unwrap();

                stability_pool_operations::update_pending_gains(
                    &mut stability_pool_state,
                    &mut user_one,
                    &mut epoch_to_scale_to_sum,
                )
                .unwrap();
            }
        }

        let first_user_hbb_gains = user_one.pending_gains_per_user.hbb;
        let total_hbb_issuance = stability_pool_state.pending_collateral_gains.hbb;
        let coll_error_hbb = stability_pool_state.last_coll_loss_error_offset.hbb;

        println!("User 1 {}", first_user_hbb_gains);
        println!("Total {}", total_hbb_issuance);
        println!(
            "Total error {}",
            coll_error_hbb / (DECIMAL_PRECISION as u64)
        );

        // 0.3 hbb precision loss per year, not too bad
        let acceptable_error = (HBB_FACTOR * 3 / 10) as u64;
        assert_fuzzy_eq!(
            first_user_hbb_gains,
            (TOTAL_HBB_TO_STABILITY_POOL * HBB_FACTOR / 2) as u128,
            acceptable_error
        );
    }

    #[test]
    fn test_hbb_issuance_one_user_many_years() {
        for i in 1..5 {
            let when = SECONDS_PER_YEAR * i;
            let expected_issuance = ((TOTAL_HBB_TO_STABILITY_POOL as f64)
                * (1.0 - 0.5_f64.powf(i as f64))
                * (HBB_FACTOR as f64)) as u128;

            let mut stability_pool_state = StabilityPoolState::default();
            let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
            let liquidations = RefCell::new(LiquidationsQueue::default());
            let hbb_emissions_start_ts = 0;

            stability_pool_operations::initialize_stability_pool(
                &mut stability_pool_state,
                &mut liquidations.borrow_mut(),
                hbb_emissions_start_ts,
            );

            let mut user_one = StabilityProviderState::default();
            stability_pool_operations::approve_new_user(&mut stability_pool_state, &mut user_one);

            // This should trigger no issuance, there is nothing there yet
            stability_pool_operations::provide_stability(
                &mut stability_pool_state,
                &mut user_one,
                &mut epoch_to_scale_to_sum,
                sol_to_lamports(100.0),
                SECONDS_PER_DAY,
            )
            .unwrap();

            // This should trigger first issuance
            // as there is only one user in the pool, he gets the full hbb issuance of first 2 DAYS of
            stability_pool_operations::provide_stability(
                &mut stability_pool_state,
                &mut user_one,
                &mut epoch_to_scale_to_sum,
                sol_to_lamports(0.000000001),
                when as u64,
            )
            .unwrap();

            stability_pool_operations::update_pending_gains(
                &mut stability_pool_state,
                &mut user_one,
                &mut epoch_to_scale_to_sum,
            )
            .unwrap();

            let first_user_hbb_gains = user_one.pending_gains_per_user.hbb;
            let total_hbb_issuance = stability_pool_state.pending_collateral_gains.hbb;

            // numbers are huge, 200 represents 0.0000000..
            assert_fuzzy_eq!(first_user_hbb_gains, expected_issuance, 200);
            assert_fuzzy_eq!(total_hbb_issuance, expected_issuance, 200);
        }
    }
}
