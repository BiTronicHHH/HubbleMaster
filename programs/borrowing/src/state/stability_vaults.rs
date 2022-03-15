use anchor_lang::prelude::Pubkey;

use crate::state::CollateralToken;
use crate::{StabilityToken, StabilityVaults};

impl StabilityVaults {
    pub fn vault_address(&self, token: StabilityToken) -> Pubkey {
        match token {
            StabilityToken::ETH => self.liquidation_rewards_vault_eth,
            StabilityToken::BTC => self.liquidation_rewards_vault_btc,
            StabilityToken::SRM => self.liquidation_rewards_vault_srm,
            StabilityToken::RAY => self.liquidation_rewards_vault_ray,
            StabilityToken::FTT => self.liquidation_rewards_vault_ftt,
            StabilityToken::SOL => self.liquidation_rewards_vault_sol,
            _ => unimplemented!(),
        }
    }

    pub fn vault_address_for_collateral_token(&self, token: CollateralToken) -> Pubkey {
        match token {
            CollateralToken::ETH => self.liquidation_rewards_vault_eth,
            CollateralToken::BTC => self.liquidation_rewards_vault_btc,
            CollateralToken::SRM => self.liquidation_rewards_vault_srm,
            CollateralToken::RAY => self.liquidation_rewards_vault_ray,
            CollateralToken::FTT => self.liquidation_rewards_vault_ftt,
            CollateralToken::SOL => self.liquidation_rewards_vault_sol,
        }
    }
}
