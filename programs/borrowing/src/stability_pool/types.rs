use crate::{StabilityCollateralAmounts, StabilityTokenMap};
#[derive(Debug)]
pub struct ProvideStabilityEffects {
    pub usd_to_stability_pool_transfer: u64,
}

#[derive(Debug)]
pub struct WithdrawStabilityEffects {
    pub usd_remaining_to_withdraw: u64,
}

#[derive(Debug)]
pub struct HarvestLiquidationGainsEffects {
    pub gains: StabilityCollateralAmounts,
}

pub struct RewardDistributionCalculation {
    pub actual_gains_considering_precision_loss: StabilityCollateralAmounts,
    // TODO: fix this, same with usd losses, ensure they match
    pub coll_gained_per_unit_staked: StabilityTokenMap,
    pub usd_loss_per_unit_staked: u64,
    pub last_coll_error: StabilityCollateralAmounts,
    pub last_usd_error: u64,
}
