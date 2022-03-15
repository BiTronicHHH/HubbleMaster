use anchor_lang::prelude::Pubkey;

use crate::state::StabilityToken;
use crate::{BorrowingVaults, CollateralToken};

impl BorrowingVaults {
    pub fn vault_address(&self, token: CollateralToken) -> Pubkey {
        match token {
            CollateralToken::ETH => self.collateral_vault_eth,
            CollateralToken::BTC => self.collateral_vault_btc,
            CollateralToken::SRM => self.collateral_vault_srm,
            CollateralToken::RAY => self.collateral_vault_ray,
            CollateralToken::FTT => self.collateral_vault_ftt,
            CollateralToken::SOL => self.collateral_vault_sol,
        }
    }

    pub fn mint_address(&self, token: CollateralToken) -> Pubkey {
        match token {
            CollateralToken::ETH => self.eth_mint,
            CollateralToken::BTC => self.btc_mint,
            CollateralToken::SRM => self.srm_mint,
            CollateralToken::RAY => self.ray_mint,
            CollateralToken::FTT => self.ftt_mint,
            _ => unimplemented!(),
        }
    }

    pub fn mint_address_for_stability_token(&self, token: StabilityToken) -> Pubkey {
        match token {
            StabilityToken::ETH => self.eth_mint,
            StabilityToken::BTC => self.btc_mint,
            StabilityToken::SRM => self.srm_mint,
            StabilityToken::RAY => self.ray_mint,
            StabilityToken::FTT => self.ftt_mint,
            _ => unimplemented!(),
        }
    }
}
