use crate::{utils::consts::ONE, DepositSnapshot, StabilityTokenMap};

impl DepositSnapshot {
    pub fn default() -> Self {
        Self {
            sum: StabilityTokenMap::default(),
            product: ONE,
            scale: 0,
            epoch: 0,
            enabled: false,
        }
    }

    pub fn new(sum: StabilityTokenMap, product: u128, scale: u64, epoch: u64) -> Self {
        Self {
            sum,
            product,
            scale,
            epoch,
            enabled: true,
        }
    }
}
