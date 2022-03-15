use crate::{StabilityPoolState, StabilityTokenMap};

impl StabilityPoolState {
    pub fn new(
        num_users: u64,
        total_users_providing_stability: u64,
        cumulative_gains_total: StabilityTokenMap,
        pending_collateral_gains: StabilityTokenMap,
        current_epoch: u64,
        current_scale: u64,
    ) -> StabilityPoolState {
        StabilityPoolState {
            num_users,
            total_users_providing_stability,
            cumulative_gains_total,
            pending_collateral_gains,
            current_epoch,
            current_scale,
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub fn to_state_string(&self) -> String {
        format!(
            "StabilityPoolState {{
    num_users: {},
    p: {},
    current_epoch: {:?},
    current_scale: {:?},
    total_usd_deposits: {},
    cumulative_gains_total: {:?},
    pending_collateral_gains: {:?},
    last_usd_error: {},
    last_coll_error: {:?}
}}
",
            self.num_users,
            self.p,
            self.current_epoch,
            self.current_scale,
            self.stablecoin_deposited,
            self.cumulative_gains_total,
            self.pending_collateral_gains,
            self.last_stablecoin_loss_error_offset,
            self.last_coll_loss_error_offset,
        )
    }
}
