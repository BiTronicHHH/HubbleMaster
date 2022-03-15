#![allow(unaligned_references)]
#[cfg(test)]
mod tests {

    use solana_sdk::native_token::sol_to_lamports;

    use crate::{
        borrowing_market::{
            borrowing_operations,
            borrowing_rate::{
                calc_borrowing_fee, calc_redemption_fee, decay_base_rate, increase_base_rate,
                refresh_base_rate, FeeEvent,
            },
            tests_utils::utils::{
                new_borrower, new_borrowing_users_with_amounts,
                new_borrowing_users_with_amounts_and_price,
            },
            types::BorrowStablecoinEffects,
        },
        redemption::test_redemptions::utils::{
            add_fill_and_clear_order, setup_redemption_borrowing_program,
        },
        state::CollateralToken,
        utils::{
            coretypes::{SOL, USDH},
            finance::CollateralInfo,
        },
        BorrowingMarketState, CollateralAmounts, StakingPoolState, TokenPrices, UserMetadata,
    };
    // ## Tests borrowing fee
    // - [x] 50 bps normal mode
    // - [x] zero during recovery mode
    // - [x] decays correctly (values)
    // - [x] decay formula is correct
    // - [x] add tests regarding base rate during recovery more (borrowing)
    // - [x] prop based - it's within range (0.5% and 5%)

    #[test]
    fn test_borrowing_fee_50_bps_normal_mode() {
        let mut market = BorrowingMarketState::new();
        let mut staking_pool = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut user = UserMetadata::default();
        let prices = TokenPrices::new(1.0);
        let now = 0;

        let (deposit, token) = (sol_to_lamports(10000.0), CollateralToken::SOL);
        let borrow = USDH::from(1000.0);

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        borrowing_operations::approve_trove(&mut market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(&mut market, &mut user, deposit, token).unwrap();
        let BorrowStablecoinEffects {
            amount_mint_to_user,
            amount_mint_to_fees_vault,
            amount_mint_to_treasury_vault,
        } = borrowing_operations::borrow_stablecoin(
            &mut market,
            &mut user,
            &mut staking_pool,
            borrow,
            &prices,
            now,
        )
        .unwrap();

        // 50 bps
        let expected_fee = borrow * 50 / 10_000;
        let treasury_fee = expected_fee * 1_500 / 10_000;
        let staking_fee = expected_fee - treasury_fee;
        assert_eq!(amount_mint_to_user, borrow);
        assert_eq!(amount_mint_to_fees_vault, staking_fee);
        assert_eq!(amount_mint_to_treasury_vault, treasury_fee);
        assert_eq!(user.borrowed_stablecoin, borrow + expected_fee);
    }

    #[test]
    fn test_borrowing_fee_0_bps_recovery_mode() {
        let mut market = BorrowingMarketState::new();
        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        let mut staking_pool = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let now = 0;

        let deposit = sol_to_lamports(1000.0);
        let borrow = USDH::from(1000.0);

        // first user, CR: 200%
        new_borrower(
            &mut market,
            &mut staking_pool,
            deposit,
            borrow,
            &TokenPrices::new(2.0),
            now,
        );

        // second user, prices dropped, CR: 140%
        let (user, effects) = new_borrower(
            &mut market,
            &mut staking_pool,
            deposit * 10,
            borrow,
            &TokenPrices::new(1.4),
            now,
        );

        let expected_fee = 0;
        assert_eq!(effects.amount_mint_to_user, borrow);
        assert_eq!(effects.amount_mint_to_fees_vault, expected_fee);
        assert_eq!(user.borrowed_stablecoin, borrow + expected_fee);
    }

    #[test]
    fn test_borrowing_fee_decays_correctly() {
        let mut market = BorrowingMarketState::new();
        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        let mut spool = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let start = 0;
        let hour = 60 * 60;

        let dep = sol_to_lamports(1000.0);
        let borrow = USDH::from(1000.0);
        let px = TokenPrices::new(2.0);

        // first user, CR: 200%
        new_borrower(&mut market, &mut spool, dep, borrow, &px, start);
        assert_eq!(market.base_rate_bps, 0);
        assert_eq!(market.last_fee_event, 0);

        // set it high
        market.base_rate_bps = 100;
        market.last_fee_event = start + 24 * hour;
        new_borrower(&mut market, &mut spool, dep, borrow, &px, start + 24 * hour);
        assert_eq!(market.base_rate_bps, 100);
        assert_eq!(market.last_fee_event, start + 24 * hour);

        // assert it (almost) halves
        new_borrower(&mut market, &mut spool, dep, borrow, &px, start + 36 * hour);
        assert_eq!(market.base_rate_bps, 49);
        assert_eq!(market.last_fee_event, start + 36 * hour);

        // assert it (almost) halves
        new_borrower(&mut market, &mut spool, dep, borrow, &px, start + 48 * hour);
        assert_eq!(market.base_rate_bps, 24);
        assert_eq!(market.last_fee_event, start + 48 * hour);

        // assert it decays to 0
        new_borrower(&mut market, &mut spool, dep, borrow, &px, 1000000);
        assert_eq!(market.base_rate_bps, 0);
        assert_eq!(market.last_fee_event, 1000000);
    }

    #[test]
    fn test_borrowing_fee_formula_is_correct() {
        let start = 0;
        let hour = 60 * 60;
        let test_cases = vec![
            (100, start, start, 100),
            (100, start, start + 12 * hour, 49),
            (100, start, start + 24 * hour, 24),
            (100, start, start + 1000000, 0),
            (10_000, start, start + 12 * hour, 4_999),
            (10_000, start, start + 24 * hour, 2_499),
        ];

        test_cases
            .into_iter()
            .for_each(|(base_rate, last_event, now, expected)| {
                let actual = decay_base_rate(base_rate, last_event, now);
                assert_eq!(actual, expected);
            });
    }

    #[test]
    fn test_borrowing_fee_decay_noop() {
        let rate = 50; // bps
        let new_rate = decay_base_rate(rate, 0, 0);
        println!("New rate {:?}", new_rate);
        assert_eq!(rate, new_rate);
    }

    #[test]
    fn test_borrowing_fee_decay_half() {
        let rate = 50; // bps
        let now = 12 * 60 * 60; // twelve hours
        let new_rate = decay_base_rate(rate, 0, now);
        println!("New rate {:?}", new_rate);
        // 24
        assert_eq!(rate / 2 - 1, new_rate);
    }

    #[test]
    fn test_borrowing_fee_decay_almost_zero() {
        let rate = 50; // bps
        let now = 2 * 24 * 60 * 60;
        // two days
        // 12h -> 25
        // 24h -> 12.5
        // 36h -> 6.75 / 2 = 3.375
        // 48h -> 3.375 -> to_int == 3
        let new_rate = decay_base_rate(rate, 0, now);
        println!("New rate {:?}", new_rate);
        assert_eq!(new_rate, 3);
    }

    #[test]
    fn test_borrowing_fee_increase_none() {
        let rate = 50;
        let supply = 10000000;
        let redeemed = 1;

        let new_rate = increase_base_rate(rate, supply, redeemed).unwrap();

        println!("New rate {:?}", new_rate);
        assert_eq!(new_rate, rate);
    }

    #[test]
    fn test_borrowing_fee_increase_one_percent() {
        let rate = 50;
        let supply = 100;
        let redeemed = 1;

        // 1/100 is one percent of the supply
        // divide by half -> 0.5% -> 50 bps
        let new_rate = increase_base_rate(rate, supply, redeemed).unwrap();

        println!("New rate {:?}", new_rate);
        assert_eq!(new_rate, rate + 50);
    }

    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn test_borrowing_fee_is_in_range(base_rate: u16, last_fee_event: u64, now: u64) -> bool {
        let base_rate = decay_base_rate(base_rate, last_fee_event, now);
        let calculated = calc_borrowing_fee(base_rate);

        50 <= calculated && calculated <= 500
    }

    // ## Tests borrowing fee
    // - [x] prop based - it's within range (0.5% and 5%)
    // - [x] address tdo regarding redeeming if undercollateralized
    // - [x] redemption rate assertions (in the right bands)
    // - [x] redemption: ensure when redeeming the user is well collateralized
    // - [x] redemption: // Find the first trove with ICR >= MCR
    // - [x] assert redemption constraints - cannot redeem TCR < MCR _requireTCRoverMCR
    // - [x] redemption: _updateStakeAndTotalStakes
    // - [x] add redemption bootstrap period tests
    // - [x] add tests regarding base rate during recovery more (borrowing)
    // - [x] assert redemption is allwoed when is necessary
    // - [x] 50 bps normal mode + amount/supply
    // - [x] 50 bps recovery mode + amount/supply
    // - [x] add tests regarding base rate during recovery more (redemptiom)
    // - [x] test redemption rates update correctly
    // - [x] decays based on last fee then it increases by amount correctly (values)
    // - [x] same during recovery mode
    // - [x] test redemption rate updates at the right points in execution path
    // - [x] increase formula is correct
    // - [ ] _requireNoUnderCollateralizedTroves

    #[quickcheck]
    fn test_redemption_fee_is_in_range(
        base_rate: u16,
        last_fee_event: u64,
        now: u64,
        redeeming: u64,
        supply: u64,
    ) -> bool {
        if redeeming == 0 || supply == 0 {
            // testing this case in the test below
            true
        } else {
            let mut market = BorrowingMarketState::default();
            borrowing_operations::initialize_borrowing_market(&mut market, 0);
            market.base_rate_bps = base_rate;
            market.last_fee_event = last_fee_event;

            let event = FeeEvent::Redemption {
                redeeming,
                supply: u64::max(redeeming, supply),
            };

            refresh_base_rate(&mut market, event, now).unwrap();
            let calculated = calc_redemption_fee(market.base_rate_bps);

            50 <= calculated && calculated <= 10_000
        }
    }

    #[test]
    fn test_redemption_fee_zeroes() {
        let base_rate: u16 = 0;
        let last_fee_event: u64 = 0;
        let now: u64 = 0;

        for (redeeming, supply) in [(0, 0), (100, 0), (0, 100)].iter() {
            let mut market = BorrowingMarketState::default();
            borrowing_operations::initialize_borrowing_market(&mut market, 0);
            market.base_rate_bps = base_rate;
            market.last_fee_event = last_fee_event;

            let event = FeeEvent::Redemption {
                redeeming: *redeeming,
                supply: *supply,
            };

            let res = refresh_base_rate(&mut market, event, now);
            assert_eq!(res.err().unwrap(), crate::BorrowError::ZeroAmountInvalid);
        }
    }

    fn to_coll_map(amts: &[f64]) -> Vec<CollateralAmounts> {
        amts.iter()
            .map(|amt| CollateralAmounts::of_token_f64(*amt, CollateralToken::SOL))
            .collect()
    }

    #[test]
    fn test_redemption_fee_based_on_burned_normal() {
        let (mut market, mut staking_pool_state, redemptions_queue, _) =
            setup_redemption_borrowing_program();

        let now_timestamp = 0;
        let count = 2;
        let borrow = USDH::from(5000.0);
        let redeem_amount = USDH::from(2000.0);

        let mut borrowers = new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            count,
            &vec![borrow; count],
            &to_coll_map(&[20000.0, 80000.0]),
            now_timestamp,
        );

        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(1.0),
            redeem_amount,
            now_timestamp,
        )
        .unwrap();

        // base_rate increased
        // we're burning 2000.0 out of 10050.0 -> 2000 / 10050 = 0.19900497512437812 / 2 = 0.09950248756218906 -> 10%
        assert_eq!(market.base_rate_bps, 995);
    }

    #[test]
    fn test_redemption_fee_based_on_burned_recovery_mode() {
        let (mut market, mut staking_pool_state, redemptions_queue, _) =
            setup_redemption_borrowing_program();

        let now_timestamp = 0;
        let count = 2;
        let borrow = USDH::from(5000.0);
        let redeem_amount = USDH::from(2000.0);

        let mut borrowers = new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            count,
            &vec![borrow; count],
            &to_coll_map(&[5000.0, 5000.0]),
            now_timestamp,
        );

        let tcr = CollateralInfo::calc_coll_ratio(
            market.stablecoin_borrowed,
            &market.deposited_collateral,
            &TokenPrices::new(1.3),
        )
        .to_percent()
        .unwrap();
        println!("TCR {}%", tcr);
        assert!(tcr < 150);

        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(1.3),
            redeem_amount,
            now_timestamp,
        )
        .unwrap();

        // base_rate increased
        // we're burning 2000.0 out of 10050.0 -> 2000 / 10050 = 0.19900497512437812 / 2 = 0.09950248756218906 -> 10%
        // recovery mode has no impact
        assert_eq!(market.base_rate_bps, 995);
        assert_eq!(market.stablecoin_borrowed, USDH::from(8050.0));
    }

    #[test]
    fn test_redemption_fee_based_on_burned_recovery_borrow_and_redeem() {
        let (mut market, mut staking_pool_state, redemptions_queue, _) =
            setup_redemption_borrowing_program();

        let now_timestamp = 0;
        let count = 2;
        let borrow = USDH::from(5000.0);
        let redeem_amount = USDH::from(2000.0);

        let mut borrowers = new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            count,
            &vec![borrow; count],
            &to_coll_map(&[5000.0, 5000.0]),
            now_timestamp,
        );

        let tcr = CollateralInfo::calc_coll_ratio(
            market.stablecoin_borrowed,
            &market.deposited_collateral,
            &TokenPrices::new(1.3),
        )
        .to_percent()
        .unwrap();
        println!("TCR {}%", tcr);
        assert!(tcr < 150);

        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(1.3),
            redeem_amount,
            now_timestamp,
        )
        .unwrap();

        // base_rate increased
        // we're burning 2000.0 out of 10050.0 -> 2000 / 10050 = 0.19900497512437812 / 2 = 0.09950248756218906 -> 10%
        // recovery mode has no impact
        assert_eq!(market.base_rate_bps, 995);
        assert_eq!(market.stablecoin_borrowed, USDH::from(8050.0));

        let tcr = CollateralInfo::calc_coll_ratio(
            market.stablecoin_borrowed,
            &market.deposited_collateral,
            &TokenPrices::new(1.1),
        )
        .to_percent()
        .unwrap();
        println!("TCR {}%", tcr);
        assert!(tcr < 150);

        // Borrowing at 0 rate
        // new borrowers
        // 5000 * 1.6 = 8000 / 1.1 = 7272.727272727272
        // let current_base_rate =
        let _ = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            1,
            &vec![borrow; 1],
            &to_coll_map(&[7200.0]),
            1.1,
            now_timestamp, // 12 hours
        );

        // borrow at 0 rate
        assert_eq!(market.stablecoin_borrowed, USDH::from(8050.0) + borrow);

        let tcr = CollateralInfo::calc_coll_ratio(
            market.stablecoin_borrowed,
            &market.deposited_collateral,
            &TokenPrices::new(1.1),
        )
        .to_percent()
        .unwrap();
        println!("TCR {}%", tcr);

        // assert redemption rate is still high
        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(1.3),
            redeem_amount,
            now_timestamp,
        )
        .unwrap();

        // base_rate increased
        // we're burning 2000.0 out of 13050.0 -> 2000 / 13050  = 0.1532567049808429 / 2 = 0.07662835249042145 -> 7.66%
        // recovery mode has no impact
        assert_eq!(market.base_rate_bps, 995 + 766);
        assert_eq!(
            market.stablecoin_borrowed,
            USDH::from(8050.0) + borrow - redeem_amount
        );
    }

    #[test]
    fn test_redemption_fee_based_on_burned_subsequent_borrowing_and_redemptions() {
        let (mut market, mut staking_pool_state, redemptions_queue, _) =
            setup_redemption_borrowing_program();

        let now_timestamp = 0;
        let count = 2;
        let borrow = USDH::from(5000.0);
        let redeem_amount = USDH::from(2000.0);

        let mut borrowers = new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            count,
            &vec![borrow; count],
            &to_coll_map(&[20000.0, 80000.0]),
            now_timestamp,
        );

        // Burn 2000.0 out of 10050.0
        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(1.0),
            redeem_amount,
            now_timestamp,
        )
        .unwrap();
        assert_eq!(market.base_rate_bps, 995);

        // Burn 2000.0 out of 8050.0
        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(1.0),
            redeem_amount,
            now_timestamp,
        )
        .unwrap();

        // base_rate increased
        // we're burning 2000.0 out of 8050.0 -> 2000 / 8050 = 0.2484472049689441 / 2 = 0.12422360248447205 -> 12.4%
        assert_eq!(market.base_rate_bps, 995 + 1242);

        // Burn 2000.0 out of 6050.0
        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(1.0),
            redeem_amount,
            now_timestamp,
        )
        .unwrap();
        // base_rate increased
        // we're burning 2000.0 out of 6050.0 -> 2000 / 6050 = 0.3305785123966942 / 2 = 0.1652892561983471 -> 16.52%
        assert_eq!(market.base_rate_bps, 995 + 1242 + 1652);

        // new borrowers
        let mut new_borrowers = new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            count,
            &vec![borrow; count],
            &to_coll_map(&[20000.0, 80000.0]),
            now_timestamp + 12 * 60 * 60, // 12 hours
        );

        // Should be halved
        // (995 + 1242 + 1652) / 2 = 1944
        // new total debt is 10000 * min(5%, 19.44%)
        assert_eq!(market.base_rate_bps, (995 + 1242 + 1652) / 2);
        assert_eq!(market.base_rate_bps, 1944);

        // Burn 2000.0 out of 4050.0 + 10500.0 = 14550
        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut new_borrowers,
            &TokenPrices::new(1.0),
            redeem_amount,
            now_timestamp + 24 * 60 * 60, // 24 hours
        )
        .unwrap();

        // firstly it decays by half then it's increased
        // 1944 / 2 = 972 ~ 971
        // we're burning 2000.0 out of  14550 -> 2000 /  14550 = 0.13745704467353953 / 2 = 0.06872852233676977 -> 6.87%
        assert_eq!(market.base_rate_bps, 971 + 687);
    }

    #[test]
    fn test_full_redemption_coll_surplus_is_inactive() {
        let (mut market, mut staking_pool_state, redemptions_queue, _) =
            setup_redemption_borrowing_program();

        let now_timestamp = 0;
        let count = 2;
        let borrow = USDH::from(2000.0);
        let redeem_amount = USDH::from(3000.0);

        let mut borrowers = new_borrowing_users_with_amounts(
            &mut market,
            &mut staking_pool_state,
            count,
            &vec![borrow; count],
            &to_coll_map(&[20000.0, 80000.0]),
            now_timestamp,
        );

        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(1.0),
            redeem_amount,
            now_timestamp,
        )
        .unwrap();

        // 2000 * 1.005 = 2010.0
        // 3000 - 2010.0 = 990
        // 2010.0 - (3000 - 2010.0) = 1020
        assert_eq!(borrowers[0].borrowed_stablecoin, 0);
        assert_eq!(borrowers[1].borrowed_stablecoin, USDH::from(1020.0));
        // user1 lost 2010.0 in collateral ->
        assert_eq!(
            borrowers[0].inactive_collateral.sol,
            SOL::from(20000.0 - 2010.0)
        );

        assert_eq!(
            borrowers[1].deposited_collateral.sol,
            SOL::from(80000.0 - 990.0)
        );

        // All of borrowers[0]'s collateral since
        // part of it goes to the redeemers + bots
        // and the rest is backing 0 debt, so it turns inactive
        assert_eq!(market.inactive_collateral.sol, SOL::from(20000.0 + 990.0));
        assert_eq!(market.deposited_collateral.sol, SOL::from(80000.0 - 990.0));
    }
}
