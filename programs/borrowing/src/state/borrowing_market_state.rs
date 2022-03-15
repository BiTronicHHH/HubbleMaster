use crate::BorrowingMarketState;

impl BorrowingMarketState {
    pub fn new() -> BorrowingMarketState {
        BorrowingMarketState {
            ..Default::default()
        }
    }

    pub fn to_state_string(&self) -> String {
        format!(
            "BorrowingMarketState {{
    num_users: {},
    stablecoin_borrowed: {},
    deposited_collateral: {:?},
    base_rate: {:?},
    redistributed_stablecoin: {:?},
    last_fee_event: {:?},
    total_stake: {:?},
    collateral_reward_per_token: {:?},
    stablecoin_reward_per_token: {:?},
    total_stake_snapshot: {:?},
    borrowed_stablecoin_snapshot: {:?},
}}
",
            self.num_users,
            self.stablecoin_borrowed,
            self.deposited_collateral,
            self.base_rate_bps,
            self.redistributed_stablecoin,
            self.last_fee_event,
            self.total_stake,
            self.collateral_reward_per_token,
            self.stablecoin_reward_per_token,
            self.total_stake_snapshot,
            self.borrowed_stablecoin_snapshot,
        )
    }
}
