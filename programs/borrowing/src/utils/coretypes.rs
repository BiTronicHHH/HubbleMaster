#[cfg(test)]
use super::consts::{
    BTC_PYTH_EXPONENT, ETH_PYTH_EXPONENT, FTT_PYTH_EXPONENT, RAY_PYTH_EXPONENT, SOL_PYTH_EXPONENT,
    SRM_PYTH_EXPONENT,
};
use crate::{state::CollateralToken, BorrowError, Price, TokenPrices};
use std::fmt;

pub trait CheckedAssign {
    fn checked_add_assign(&mut self, rhs: Self) -> Result<(), BorrowError>;
    fn checked_sub_assign(&mut self, rhs: Self) -> Result<(), BorrowError>;
}

impl CheckedAssign for u64 {
    fn checked_add_assign(&mut self, rhs: Self) -> Result<(), BorrowError> {
        *self = self.checked_add(rhs).ok_or(BorrowError::MathOverflow)?;
        Ok(())
    }
    fn checked_sub_assign(&mut self, rhs: Self) -> Result<(), BorrowError> {
        *self = self.checked_sub(rhs).ok_or(BorrowError::MathOverflow)?;
        Ok(())
    }
}

impl CheckedAssign for u128 {
    fn checked_add_assign(&mut self, rhs: Self) -> Result<(), BorrowError> {
        *self = self.checked_add(rhs).ok_or(BorrowError::MathOverflow)?;
        Ok(())
    }
    fn checked_sub_assign(&mut self, rhs: Self) -> Result<(), BorrowError> {
        *self = self.checked_sub(rhs).ok_or(BorrowError::MathOverflow)?;
        Ok(())
    }
}

impl Price {
    pub fn from(value: u64, exp: u8) -> Self {
        Price { value, exp }
    }

    pub fn f64(&self) -> f64 {
        (self.value as f64) / 10_f64.powf(self.exp as f64)
    }

    #[cfg(test)]
    pub fn from_f64(price: f64, token: CollateralToken) -> Price {
        let exponent = match token {
            CollateralToken::SOL => SOL_PYTH_EXPONENT,
            CollateralToken::ETH => ETH_PYTH_EXPONENT,
            CollateralToken::BTC => BTC_PYTH_EXPONENT,
            CollateralToken::SRM => SRM_PYTH_EXPONENT,
            CollateralToken::RAY => RAY_PYTH_EXPONENT,
            CollateralToken::FTT => FTT_PYTH_EXPONENT,
        };

        let val = (price * 10_f64.powf(exponent as f64)) as u64;
        Self::from(val, exponent)
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.f64();
        f.write_str(&format!("px={}", &val))
    }
}

impl TokenPrices {
    #[cfg(test)]
    pub fn new(sol_price: f64) -> Self {
        TokenPrices {
            sol: Price::from(
                (sol_price * 10_f64.powf(SOL_PYTH_EXPONENT as f64)) as u64,
                SOL_PYTH_EXPONENT,
            ),
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub fn new_all(price: f64) -> Self {
        TokenPrices {
            sol: Price::from(
                (price * 10_f64.powf(SOL_PYTH_EXPONENT as f64)) as u64,
                SOL_PYTH_EXPONENT,
            ),
            eth: Price::from(
                (price * 10_f64.powf(ETH_PYTH_EXPONENT as f64)) as u64,
                ETH_PYTH_EXPONENT,
            ),
            btc: Price::from(
                (price * 10_f64.powf(BTC_PYTH_EXPONENT as f64)) as u64,
                BTC_PYTH_EXPONENT,
            ),
            srm: Price::from(
                (price * 10_f64.powf(SRM_PYTH_EXPONENT as f64)) as u64,
                SRM_PYTH_EXPONENT,
            ),
            ray: Price::from(
                (price * 10_f64.powf(RAY_PYTH_EXPONENT as f64)) as u64,
                RAY_PYTH_EXPONENT,
            ),
            ftt: Price::from(
                (price * 10_f64.powf(FTT_PYTH_EXPONENT as f64)) as u64,
                FTT_PYTH_EXPONENT,
            ),
        }
    }

    pub fn token_amount(&self, token: CollateralToken) -> Price {
        match token {
            CollateralToken::SOL => self.sol,
            CollateralToken::ETH => self.eth,
            CollateralToken::BTC => self.btc,
            CollateralToken::SRM => self.srm,
            CollateralToken::RAY => self.ray,
            CollateralToken::FTT => self.ftt,
        }
    }
}

pub struct USDH;
pub struct HBB;
pub struct SOL;

impl USDH {
    #[cfg(test)]
    pub fn from(amount: f64) -> u64 {
        use super::math;

        math::stablecoin_decimal_to_u64(amount)
    }
}

impl HBB {
    #[cfg(test)]
    pub fn from(amount: f64) -> u64 {
        use super::math;

        math::hbb_decimal_to_u64(amount)
    }
}

impl SOL {
    #[cfg(test)]
    pub fn from(amount: f64) -> u64 {
        use super::math::coll_to_lamports;

        coll_to_lamports(amount, CollateralToken::SOL)
    }
}
