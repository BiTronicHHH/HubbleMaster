use crate::CollateralAmounts;

#[derive(Debug, Default, Clone)]
pub struct RedemptionCollateralSplit {
    pub filler: CollateralAmounts,
    pub clearer: CollateralAmounts,
    pub redeemer: CollateralAmounts,
    pub stakers: CollateralAmounts,
    pub total: CollateralAmounts,
}

impl RedemptionCollateralSplit {
    pub fn checked_add_assign(&mut self, other: &RedemptionCollateralSplit) {
        self.filler.add_assign(&other.filler);
        self.clearer.add_assign(&other.clearer);
        self.redeemer.add_assign(&other.redeemer);
        self.stakers.add_assign(&other.stakers);
        self.total.add_assign(&other.total);
    }
}

#[derive(Debug)]
pub struct RedemptionFillingResults {
    pub collateral_redeemed: RedemptionCollateralSplit,
    pub collateral_made_inactive: CollateralAmounts,
    pub debt_redeemed: u64,
}

#[derive(Debug, Clone)]
pub struct AddRedemptionOrderEffects {
    pub redemption_order_id: u64,
    pub transfer_stablecoin_amount: u64,
}

#[derive(Debug, Clone)]
pub struct ClearRedemptionOrderEffects {
    pub redeemed_stablecoin: u64,
    pub redeemed_collateral: RedemptionCollateralSplit,
}
