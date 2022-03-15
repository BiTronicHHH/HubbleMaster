use super::{StabilityCollateralAmounts, StabilityToken, StabilityTokenMap};

impl StabilityTokenMap {
    pub fn is_zero_token(&self, token: StabilityToken) -> bool {
        match token {
            StabilityToken::SOL => self.sol == 0,
            StabilityToken::ETH => self.eth == 0,
            StabilityToken::BTC => self.btc == 0,
            StabilityToken::SRM => self.srm == 0,
            StabilityToken::RAY => self.ray == 0,
            StabilityToken::FTT => self.ftt == 0,
            StabilityToken::HBB => self.hbb == 0,
        }
    }

    pub fn token_amount(&self, token: StabilityToken) -> u128 {
        match token {
            StabilityToken::SOL => self.sol,
            StabilityToken::ETH => self.eth,
            StabilityToken::BTC => self.btc,
            StabilityToken::SRM => self.srm,
            StabilityToken::RAY => self.ray,
            StabilityToken::FTT => self.ftt,
            StabilityToken::HBB => self.hbb,
        }
    }

    pub fn of_token(amount: u128, token: StabilityToken) -> Self {
        let mut new = Self::default();
        match token {
            StabilityToken::SOL => new.sol = amount,
            StabilityToken::ETH => new.eth = amount,
            StabilityToken::BTC => new.btc = amount,
            StabilityToken::SRM => new.srm = amount,
            StabilityToken::RAY => new.ray = amount,
            StabilityToken::FTT => new.ftt = amount,
            StabilityToken::HBB => new.hbb = amount,
        };
        new
    }

    pub fn to_collateral_amounts(&self) -> StabilityCollateralAmounts {
        StabilityCollateralAmounts {
            sol: self.sol as u64,
            eth: self.eth as u64,
            btc: self.btc as u64,
            srm: self.srm as u64,
            ray: self.ray as u64,
            ftt: self.ftt as u64,
            hbb: self.hbb as u64,
        }
    }
}
