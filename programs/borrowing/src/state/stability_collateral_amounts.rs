use super::{StabilityCollateralAmounts, StabilityToken, StabilityTokenMap};

impl StabilityCollateralAmounts {
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

    pub fn token_amount(&self, token: StabilityToken) -> u64 {
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

    pub fn of_token(amount: u64, token: StabilityToken) -> Self {
        let mut new = StabilityCollateralAmounts::default();
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

    pub fn to_token_map(&self) -> StabilityTokenMap {
        StabilityTokenMap {
            sol: self.sol as u128,
            eth: self.eth as u128,
            btc: self.btc as u128,
            srm: self.srm as u128,
            ray: self.ray as u128,
            ftt: self.ftt as u128,
            hbb: self.hbb as u128,
        }
    }
}
