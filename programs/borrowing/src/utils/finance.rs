use decimal_wad::{common::TryDiv, decimal::Decimal};

use crate::state::CollateralToken;
use crate::{CollateralAmounts, UserMetadata};

use super::consts::{
    BTC_DECIMALS, ETH_DECIMALS, FTT_DECIMALS, RAY_DECIMALS, SOL_DECIMALS, SRM_DECIMALS,
    USDH_DECIMALS,
};
use crate::{Price, TokenPrices};

#[derive(Debug)]
pub struct CollateralInfo {
    pub collateral_value: u64,
    pub collateral_ratio: Decimal,
    pub net_value: i64,
}

impl CollateralInfo {
    pub fn from(user: &UserMetadata, prices: &TokenPrices) -> Self {
        Self::calculate_collateral_value(
            user.borrowed_stablecoin,
            &user.deposited_collateral,
            prices,
        )
    }

    pub fn calc_coll_ratio(
        debt_usdh: u64,
        collateral_deposited: &CollateralAmounts,
        prices: &TokenPrices,
    ) -> Decimal {
        let collateral_value = Self::calc_market_value_usdh(prices, collateral_deposited);
        Self::coll_ratio(debt_usdh, collateral_value)
    }
    pub fn coll_ratio(debt_usdh: u64, market_value_usdh: u64) -> Decimal {
        if market_value_usdh == 0 && debt_usdh == 0 {
            Decimal::from(u64::MAX)
        } else if debt_usdh == 0 {
            Decimal::from(u64::MAX)
        } else {
            Decimal::from(market_value_usdh)
                .try_div(Decimal::from(debt_usdh))
                .unwrap()
        }
    }
    pub fn calculate_collateral_value(
        debt_usdh: u64,
        collateral_deposited: &CollateralAmounts,
        prices: &TokenPrices,
    ) -> CollateralInfo {
        let collateral_value = Self::calc_market_value_usdh(prices, collateral_deposited);
        let collateral_ratio = Self::calc_coll_ratio(debt_usdh, collateral_deposited, prices);

        CollateralInfo {
            collateral_value,
            collateral_ratio,
            net_value: (collateral_value as i64) - (debt_usdh as i64), // this could be negative
        }
    }

    pub fn calc_market_value_usdh(prices: &TokenPrices, amounts: &CollateralAmounts) -> u64 {
        use CollateralToken::*;
        let sol = Self::calc_market_value_token(amounts.sol, &prices.sol, SOL);
        let eth = Self::calc_market_value_token(amounts.eth, &prices.eth, ETH);
        let btc = Self::calc_market_value_token(amounts.btc, &prices.btc, BTC);
        let srm = Self::calc_market_value_token(amounts.srm, &prices.srm, SRM);
        let ray = Self::calc_market_value_token(amounts.ray, &prices.ray, RAY);
        let ftt = Self::calc_market_value_token(amounts.ftt, &prices.ftt, FTT);
        let mv = sol + eth + btc + srm + ray + ftt;
        mv as u64
    }

    pub fn calc_market_value_token(
        amount: u64,
        price: &Price,
        collateral: CollateralToken,
    ) -> u128 {
        let token_decimals = match collateral {
            CollateralToken::SOL => SOL_DECIMALS,
            CollateralToken::ETH => ETH_DECIMALS,
            CollateralToken::BTC => BTC_DECIMALS,
            CollateralToken::SRM => SRM_DECIMALS,
            CollateralToken::RAY => RAY_DECIMALS,
            CollateralToken::FTT => FTT_DECIMALS,
        };

        (amount as u128)
            .checked_mul(price.value as u128)
            .unwrap()
            .checked_div(Self::ten_pow(
                token_decimals
                    .checked_add(price.exp)
                    .unwrap()
                    .checked_sub(USDH_DECIMALS)
                    .unwrap(),
            ))
            .unwrap()
    }

    fn ten_pow(exponent: u8) -> u128 {
        let value: u128 = match exponent {
            16 => 10_000_000_000_000_000,
            15 => 1_000_000_000_000_000,
            14 => 100_000_000_000_000,
            13 => 10_000_000_000_000,
            12 => 1_000_000_000_000,
            11 => 100_000_000_000,
            10 => 10_000_000_000,
            9 => 1_000_000_000,
            8 => 100_000_000,
            7 => 10_000_000,
            6 => 1_000_000,
            5 => 100_000,
            4 => 10_000,
            3 => 1_000,
            2 => 100,
            1 => 10,
            0 => 1,
            _ => panic!("no support for exponent: {}", exponent),
        };

        value
    }
}
