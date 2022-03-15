use anchor_lang::prelude::Pubkey;

use crate::state::RedemptionsQueue;
use crate::RedemptionOrder;

pub enum RedemptionOrderStatus {
    Inactive = 0,
    Open = 1,
    Filling = 2,
    Claiming = 3,
}
pub enum RedemptionCandidateStatus {
    Inactive = 0,
    Active = 1,
}

impl From<RedemptionCandidateStatus> for u8 {
    fn from(val: RedemptionCandidateStatus) -> u8 {
        match val {
            RedemptionCandidateStatus::Inactive => 0,
            RedemptionCandidateStatus::Active => 1,
        }
    }
}

impl From<RedemptionOrderStatus> for u8 {
    fn from(val: RedemptionOrderStatus) -> u8 {
        match val {
            RedemptionOrderStatus::Inactive => 0,
            RedemptionOrderStatus::Open => 1,
            RedemptionOrderStatus::Filling => 2,
            RedemptionOrderStatus::Claiming => 3,
        }
    }
}

impl From<u8> for RedemptionOrderStatus {
    fn from(number: u8) -> Self {
        match number {
            0 => RedemptionOrderStatus::Inactive,
            1 => RedemptionOrderStatus::Open,
            2 => RedemptionOrderStatus::Filling,
            3 => RedemptionOrderStatus::Claiming,
            _ => panic!("Redemption Order Conversion"),
        }
    }
}

impl Default for RedemptionsQueue {
    #[cfg(test)]
    fn default() -> Self {
        use crate::utils::consts::MAX_REDEMPTION_EVENTS;
        let orders: [RedemptionOrder; MAX_REDEMPTION_EVENTS] =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        RedemptionsQueue {
            orders,
            next_index: 0,
        }
    }

    #[cfg(not(test))]
    fn default() -> Self {
        unimplemented!()
    }
}

impl RedemptionOrder {
    #[cfg(not(test))]
    pub fn new(_id: u64, _redeemer: Pubkey) -> Self {
        unimplemented!()
    }

    #[cfg(test)]
    pub fn new(id: u64, redeemer: Pubkey) -> Self {
        use crate::state::{CandidateRedemptionUser, TokenPrices};
        let candidate_users: [CandidateRedemptionUser; 32] =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        RedemptionOrder {
            last_reset: 0,
            id,
            redeemer,
            base_rate: 0,
            status: 0,
            redeemer_user_metadata: Pubkey::default(),
            requested_amount: 0,
            remaining_amount: 0,
            redemption_prices: TokenPrices::default(),
            candidate_users,
        }
    }

    #[cfg(test)]
    pub fn to_state_string(&self) -> String {
        format!(
            "RedemptionOrder {{
    last_reset: {},
    id: {},
    status: {:?},
    redeemer_user_metadata: {:?},
    redeemer: {},
    requested_amount: {:?},
    remaining_amount: {:?},
    redemption_prices: {:?},
    candidate_users: {:?}
}}
",
            self.last_reset,
            self.id,
            self.status,
            self.redeemer_user_metadata,
            self.redeemer,
            self.requested_amount,
            self.remaining_amount,
            self.redemption_prices,
            self.candidate_users
                .iter()
                .filter(|order| order.status == RedemptionCandidateStatus::Active as u8)
                .count(),
        )
    }
}
