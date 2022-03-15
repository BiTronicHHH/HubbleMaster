use super::{CollateralAmounts, CollateralToken, TokenMap};

impl CollateralAmounts {
    pub fn is_zero_token(&self, token: CollateralToken) -> bool {
        match token {
            CollateralToken::SOL => self.sol == 0,
            CollateralToken::ETH => self.eth == 0,
            CollateralToken::BTC => self.btc == 0,
            CollateralToken::SRM => self.srm == 0,
            CollateralToken::RAY => self.ray == 0,
            CollateralToken::FTT => self.ftt == 0,
        }
    }

    pub fn token_amount(&self, token: CollateralToken) -> u64 {
        match token {
            CollateralToken::SOL => self.sol,
            CollateralToken::ETH => self.eth,
            CollateralToken::BTC => self.btc,
            CollateralToken::SRM => self.srm,
            CollateralToken::RAY => self.ray,
            CollateralToken::FTT => self.ftt,
        }
    }

    pub fn of_token(amount: u64, token: CollateralToken) -> Self {
        let mut new = CollateralAmounts::default();
        match token {
            CollateralToken::SOL => new.sol = amount,
            CollateralToken::ETH => new.eth = amount,
            CollateralToken::BTC => new.btc = amount,
            CollateralToken::SRM => new.srm = amount,
            CollateralToken::RAY => new.ray = amount,
            CollateralToken::FTT => new.ftt = amount,
        };
        new
    }

    #[cfg(test)]
    pub fn of_token_f64(amount: f64, token: CollateralToken) -> Self {
        use crate::utils::math::coll_to_lamports;

        let mut new = CollateralAmounts::default();
        match token {
            CollateralToken::SOL => new.sol = coll_to_lamports(amount, token),
            CollateralToken::ETH => new.eth = coll_to_lamports(amount, token),
            CollateralToken::BTC => new.btc = coll_to_lamports(amount, token),
            CollateralToken::SRM => new.srm = coll_to_lamports(amount, token),
            CollateralToken::RAY => new.ray = coll_to_lamports(amount, token),
            CollateralToken::FTT => new.ftt = coll_to_lamports(amount, token),
        };
        new
    }

    pub fn to_token_map(&self) -> TokenMap {
        TokenMap {
            sol: self.sol as u128,
            eth: self.eth as u128,
            btc: self.btc as u128,
            srm: self.srm as u128,
            ray: self.ray as u128,
            ftt: self.ftt as u128,
        }
    }
}
