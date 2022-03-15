use crate::{DepositSnapshot, StabilityProviderState, StabilityTokenMap};

impl StabilityProviderState {
    pub fn approve_stability(&mut self, user_id: u64) {
        self.user_id = user_id;
        self.deposited_stablecoin = 0;
        self.user_deposit_snapshot = DepositSnapshot::default();
        self.cumulative_gains_per_user = StabilityTokenMap::default();
    }

    pub fn to_state_string(&self) -> String {
        format!(
            "StabilityProviderState {{
    user_id: {},
    user_usd_deposits: {},
    user_deposit_snapshot: {:?},
    cumulative_gains_per_user: {:?},
    pending_gains_per_user: {:?},
}}
",
            self.user_id,
            self.deposited_stablecoin,
            self.user_deposit_snapshot,
            self.cumulative_gains_per_user,
            self.pending_gains_per_user
        )
    }
}
