use crate::StakingPoolState;

impl StakingPoolState {
    pub fn new(
        total_stake: u128,
        reward_per_token: u128,
        total_distributed_rewards: u128,
        rewards_not_yet_claimed: u128,
    ) -> StakingPoolState {
        StakingPoolState {
            reward_per_token,
            total_stake,
            total_distributed_rewards,
            rewards_not_yet_claimed,
            ..Default::default()
        }
    }

    pub fn initialize_staking_pool(&mut self) {
        // Metadata for protocol analytics
        self.total_distributed_rewards = 0;
        self.rewards_not_yet_claimed = 0;

        // State data -- used to calculate rewards
        self.total_stake = 0;
        self.reward_per_token = 0;
    }

    pub fn to_state_string(&self) -> String {
        format!(
            "Stability Pool State {{
    total_stake: {},
    reward_per_token: {},
    total_distributed_rewards: {},
    rewards_not_yet_claimed: {},
}}",
            self.total_stake,
            self.reward_per_token,
            self.total_distributed_rewards,
            self.rewards_not_yet_claimed
        )
    }
}
