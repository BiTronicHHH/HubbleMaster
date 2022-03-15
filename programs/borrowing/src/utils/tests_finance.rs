#[cfg(test)]
mod tests {

    use decimal_wad::{
        common::{TryDiv, TryMul},
        decimal::Decimal,
        rate::Rate,
    };
    use solana_sdk::native_token::LAMPORTS_PER_SOL;

    use crate::{
        borrowing_market::{borrowing_rate::BorrowSplit, liquidation_calcs},
        state::CollateralToken,
        utils::{coretypes::USDH, finance::CollateralInfo, math::coll_to_lamports},
    };
    use crate::{CollateralAmounts, Price, TokenPrices};

    const HALF: f64 = 0.5;
    const MINUTE_FACTOR: f64 = 1.0 / 720.0;

    #[test]
    fn test_finance_decimal_borrow() {
        // Start from 100
        let requested_amount: u64 = 100;
        let one = Decimal::from(requested_amount);
        // One percent
        let rate = Rate::from_percent(5);

        // Get one percent
        let res = one.try_mul(rate).unwrap();

        // Scale back to the original scale
        let amount = res.try_floor_u64().unwrap();
        println!("res {:?}", amount);

        assert_eq!(amount, 5);
    }

    #[test]
    fn test_finance_decimal_bps_floor_ceil() {
        // let requested_amount: u64 = 500000;
        // Start from 100
        let requested_amount: u64 = 100;
        let one = Decimal::from(requested_amount);
        // 0.5%
        let rate = Rate::from_bps(50);

        // Get one percent
        let res = one.try_mul(rate).unwrap();

        // Scale back to the original scale
        let floor = res.try_floor_u64().unwrap();
        let ceil = res.try_ceil_u64().unwrap();
        let round = res.try_round_u64().unwrap();

        println!("res {:?}", res.try_floor_u64());
        println!("res {:?}", res.try_ceil_u64());
        println!("res {:?}", res.try_round_u64());

        assert_eq!(floor, 0);
        assert_eq!(ceil, 1);
        assert_eq!(round, 1);
    }

    #[test]
    fn test_finance_decimal_stablecoin_bps_floor_ceil() {
        let requested_amount: u64 = 500000;
        let one = Decimal::from(requested_amount);
        // 0.5%
        let rate = Rate::from_bps(50);

        // 0.005 * 500000 = 2500

        // Get one percent
        let res = one.try_mul(rate).unwrap();

        // Scale back to the original scale
        let floor = res.try_floor_u64().unwrap();
        let ceil = res.try_ceil_u64().unwrap();
        let round = res.try_round_u64().unwrap();

        println!("res {:?}", res.try_floor_u64());
        println!("res {:?}", res.try_ceil_u64());
        println!("res {:?}", res.try_round_u64());

        assert_eq!(floor, 2500);
        assert_eq!(ceil, 2500);
        assert_eq!(round, 2500);
    }

    // Calculate borrowing fee
    #[test]
    fn test_finance_fee_simple() {
        let fee_bps = 50;
        let amount = 4000000000;

        let borrow_split = BorrowSplit::split_fees(amount, fee_bps);

        println!("Borrow split {:?}", borrow_split);

        // Expectation
        // 4000 - 2000
        // >>> 3800 / 1.005
        // 3781.0945273631846
        // >>> 3781.0945273631846 * 0.005
        // 18.905472636815922
    }

    #[test]
    fn test_finance_fee_decay() {
        // Half-life of 12h. 12h = 720 min
        // (1/2) = d^720 => d = (1/2)^(1/720)

        let base_rate: f64 = 0.01;
        let minutes_passed: f64 = 12.0 * 60.0;

        // this should be const too
        let factor: f64 = HALF.powf(MINUTE_FACTOR);

        let decay = factor.powf(minutes_passed);

        println!("Prev Rate {:?}", base_rate);
        println!("Decay {:?}", decay);
        println!("New Rate {:?}", decay * base_rate);
    }

    #[test]
    fn test_finance_can_user_borrow() {
        let fee_bps = 0;
        let requested_amount = USDH::from(100.0);

        let borrow_and_fee = BorrowSplit::split_fees(requested_amount, fee_bps);

        println!("borrow_split {:?}", borrow_and_fee);

        let deposited_collateral = CollateralAmounts {
            sol: LAMPORTS_PER_SOL, // 1 SOL
            ..Default::default()
        };

        let borrowed_stablecoin = 0;
        let token_prices = TokenPrices::new(40.0);

        // prices
        // price: 7398000000
        // exponent: -9
        // 7398000000 / 1000000000 = 7.398
        // market value: 10 SRM = 10 * 7398000000 = 73980000000
        // 10 SRM = 10 * 1000000000 = 10000000000
        // market value = 10000000000 * 7398000000 = 73980000000000000000
        // 73980000000000000000 / 73980000000 = 1.000.000.000
        // 73.98 USD
        // 73980000000 / 73.98 = 1.000.000.000

        // requesting 100.0 given a collateral of 40.0, should fail
        let res = liquidation_calcs::try_borrow(
            borrow_and_fee.amount_to_borrow,
            &deposited_collateral,
            borrowed_stablecoin,
            &deposited_collateral,
            borrowed_stablecoin,
            &CollateralAmounts::default(),
            &token_prices,
            liquidation_calcs::SystemMode::Normal,
            Decimal::from_percent(150),
        );

        println!("Can borrow {:?}", res);
        assert!(res.is_err());
    }

    #[test]
    fn test_finance_prices_market_value() {
        let prices = TokenPrices {
            sol: Price {
                value: 22841550900,
                exp: 8,
            },
            eth: Price {
                value: 472659830000,
                exp: 8,
            },
            btc: Price {
                value: 6462236900000,
                exp: 8,
            },
            srm: Price {
                value: 706975570,
                exp: 8,
            },
            ray: Price {
                value: 1110038050,
                exp: 8,
            },
            ftt: Price {
                value: 591710460,
                exp: 8,
            },
        };

        let amounts = CollateralAmounts {
            sol: 1 * LAMPORTS_PER_SOL,
            eth: 0,
            btc: 0,
            srm: 0,
            ray: 0,
            ftt: 0,
        };

        let market_value_usdh = CollateralInfo::calc_market_value_usdh(&prices, &amounts);
        println!("Market Value {:?}", market_value_usdh);

        assert_eq!(market_value_usdh, USDH::from(228.41550900));

        let debt_usdh = USDH::from(100.0);
        println!("Debt {:?}", debt_usdh);

        // MV: 228.41550900
        // Debt: 100.0
        // CR: 228.41550900 / 100.0 = 2.28415509
        // MCR:                       1.1

        // Liquidatable Val: 228.41550900 / 1.1 = 207.6504627272727

        // 2284155090000000000
        // 1100000000000000000

        let collateral_ratio = Decimal::from(market_value_usdh)
            .try_div(Decimal::from(debt_usdh))
            .unwrap();

        let min_cr = Decimal::from_percent(110);
        println!("Collateral ratio {:?}", collateral_ratio);
        println!("MCR {:?}", min_cr);

        let max_amount = Decimal::from(market_value_usdh)
            .try_div(Rate::from_percent(110))
            .unwrap();

        // 228415509
        // 207650462
        println!("Max amount {:?}", max_amount.try_floor_u64());
        assert_eq!(
            max_amount.try_floor_u64().unwrap(),
            USDH::from(207.6504627272727)
        );

        // max debt amount 228.41550900 / 1.1 = 207.6504627272727
        let max_debt_amount = market_value_usdh * 100 / 110;

        assert_eq!(max_debt_amount, USDH::from(207.6504627272727));
    }

    #[test]
    fn test_finance_token_amount() {
        let usdh = USDH::from(10.0);
        println!("{}", usdh);

        let usdh = USDH::from(100.5);
        println!("{}", usdh);
    }

    #[test]
    fn test_token_map_bps() {
        let collateral = CollateralAmounts {
            sol: coll_to_lamports(10.0, CollateralToken::SOL),
            eth: coll_to_lamports(5.0, CollateralToken::ETH),
            ..Default::default()
        };

        // 3%
        let actual = collateral.mul_bps(300);

        let expected = CollateralAmounts {
            sol: coll_to_lamports(0.3, CollateralToken::SOL),
            eth: coll_to_lamports(0.15, CollateralToken::ETH),
            ..Default::default()
        };

        println!("original {:?}", collateral);
        println!("Remaining {:?}", actual);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_prices_amounts() {
        // testing decimals
        pub struct PythPrice(pub u64, pub u8);
        pub struct TokenAmount(pub u64, pub u8);
        pub struct UsdAmount(pub u128);

        pub fn market_value(token_amount: TokenAmount, price: PythPrice) -> UsdAmount {
            let usdh_exponent = 6;
            let token_amt: u128 = (token_amount.0 as u128) * 10_u128.pow(token_amount.1 as u32);
            let token_px: u128 = price.0 as u128;
            let res = token_amt * token_px
                / 10_u128.pow((token_amount.1 + price.1 - usdh_exponent) as u32);
            UsdAmount(res)
        }

        // pyth price & exponent
        let eth_px = PythPrice(472659830000_u64, 8);
        let _ray_px = PythPrice(1110038050_u64, 8);
        let _srm_px = PythPrice(706975570_u64, 8);
        let _btc_px = PythPrice(6462236900000_u64, 8);
        let sol_px = PythPrice(22841550900_u64, 8);
        let ftt_px = PythPrice(5917104600_u64, 8);

        // amount & decimals
        let requested_eth_amount = TokenAmount(2, 6);
        let requested_ray_amount = TokenAmount(5, 6);
        let requested_srm_amount = TokenAmount(10, 6);
        let requested_btc_amount = TokenAmount(1, 6);
        let requested_sol_amount = TokenAmount(2, 9);
        let requested_ftt_amount = TokenAmount(10, 6);

        let requested_eth = (requested_eth_amount.0) * 10_u64.pow(requested_eth_amount.1 as u32);
        let requested_ray = (requested_ray_amount.0) * 10_u64.pow(requested_ray_amount.1 as u32);
        let requested_srm = (requested_srm_amount.0) * 10_u64.pow(requested_srm_amount.1 as u32);
        let requested_btc = (requested_btc_amount.0) * 10_u64.pow(requested_btc_amount.1 as u32);
        let requested_sol = (requested_sol_amount.0) * 10_u64.pow(requested_sol_amount.1 as u32);
        let requested_ftt = (requested_ftt_amount.0) * 10_u64.pow(requested_ftt_amount.1 as u32);

        println!("requested_eth {}", requested_eth);
        println!("requested_ray {}", requested_ray);
        println!("requested_srm {}", requested_srm);
        println!("requested_btc {}", requested_btc);
        println!("requested_sol {}", requested_sol);
        println!("requested_ftt {}", requested_ftt);

        // 2 eth in dollars = 4726.5983 * 2 = 9453.1966 * 1000000 = 9453196600
        // 2000000 * 472659830000 = 945319660000000000
        // 945319660000000000
        // 9453196600
        // // 100 sol in dollars = 228.415509 * 100000000 = 22841550900
        // 1000000000000 * 22841550900 = 22841550900000000000000

        // price(num, exp) * amount(num, exp) / 10 ^ (p.exp + a.exp - u.exp)

        // 22841550900000000000000
        // 228415509
        // 1_000_000_000 * 2 // 2 SOL
        // 2 SOL: 228.41550900 * 2 = 456.831018 =

        let eth_dollar_amount = market_value(requested_eth_amount, eth_px);
        println!("{}", eth_dollar_amount.0);
        assert_eq!(eth_dollar_amount.0, 9453196600);

        let sol_dollar_amount = market_value(requested_sol_amount, sol_px);
        println!("{}", sol_dollar_amount.0);
        assert_eq!(sol_dollar_amount.0, 456831018);

        // 10 FTT = 59.17104600 * 10 = 591.71046
        // 10 FTT in usdh = 591710460
        let ftt_dollar_amount = market_value(requested_ftt_amount, ftt_px);
        println!("{}", ftt_dollar_amount.0);
        assert_eq!(ftt_dollar_amount.0, 591710460);

        // 1 sol in dollars =

        // what is the expected USDH amount
        // how many dollars it represents
        // 472659830000 / 100000000 = 4726.5983

        // usdh decimals
        let _usdh_amount = (1.0, 6);

        //
        let requested_amount: u64 = 100;
        let one = Decimal::from(requested_amount);
        // 0.5%
        let rate = Rate::from_bps(50);

        // amount eth
        // Get one percent
        let res = one.try_mul(rate).unwrap();

        // Scale back to the original scale
        let _floor = res.try_floor_u64().unwrap();
    }
}
