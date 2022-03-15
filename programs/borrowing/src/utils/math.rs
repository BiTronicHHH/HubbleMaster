pub use anchor_lang::solana_program::native_token::{lamports_to_sol, sol_to_lamports};

#[cfg(test)]
use crate::{utils::consts::*, CollateralToken};

#[cfg(test)]
pub fn stablecoin_decimal_to_u64(number: f64) -> u64 {
    decimal_to_u64(number, STABLECOIN_FACTOR.into())
}

#[cfg(test)]
pub fn coll_to_lamports(number: f64, collateral: CollateralToken) -> u64 {
    match collateral {
        CollateralToken::SOL => decimal_to_u64(number, 10_u128.pow(SOL_DECIMALS.into())),
        CollateralToken::ETH => decimal_to_u64(number, 10_u128.pow(ETH_DECIMALS.into())),
        CollateralToken::BTC => decimal_to_u64(number, 10_u128.pow(BTC_DECIMALS.into())),
        CollateralToken::SRM => decimal_to_u64(number, 10_u128.pow(SRM_DECIMALS.into())),
        CollateralToken::RAY => decimal_to_u64(number, 10_u128.pow(RAY_DECIMALS.into())),
        CollateralToken::FTT => decimal_to_u64(number, 10_u128.pow(FTT_DECIMALS.into())),
    }
}

#[cfg(test)]
pub fn hbb_decimal_to_u64(number: f64) -> u64 {
    decimal_to_u64(number, HBB_FACTOR.into())
}

#[cfg(test)]
pub fn decimal_to_u64(number: f64, factor: u128) -> u64 {
    let number = number * (factor as f64);
    number as u64
}

#[cfg(test)]
pub fn u64_to_decimal(number: u64, factor: u128) -> f64 {
    number as f64 / (factor as f64)
}
