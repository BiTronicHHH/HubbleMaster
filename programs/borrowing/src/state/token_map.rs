use super::{CollateralAmounts, CollateralToken, TokenMap};

impl TokenMap {
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

    pub fn token_amount(&self, token: CollateralToken) -> u128 {
        match token {
            CollateralToken::SOL => self.sol,
            CollateralToken::ETH => self.eth,
            CollateralToken::BTC => self.btc,
            CollateralToken::SRM => self.srm,
            CollateralToken::RAY => self.ray,
            CollateralToken::FTT => self.ftt,
        }
    }

    pub fn of_token(amount: u128, token: CollateralToken) -> Self {
        let mut new = TokenMap::default();
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
    pub fn to_collateral_amounts(&self) -> CollateralAmounts {
        CollateralAmounts {
            sol: self.sol as u64,
            eth: self.eth as u64,
            btc: self.btc as u64,
            srm: self.srm as u64,
            ray: self.ray as u64,
            ftt: self.ftt as u64,
        }
    }
}
