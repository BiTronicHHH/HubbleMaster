use crate::UserStakingState;

impl UserStakingState {
    pub fn to_state_string(&self) -> String {
        format!(
            "UserStakingState {{
    rewards_tally: {},
    user_stake: {}
}}",
            self.rewards_tally, self.user_stake
        )
    }
}
