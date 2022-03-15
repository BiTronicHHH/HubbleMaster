#[allow(unused_imports)]
use std::cell::Ref;

use anchor_lang::prelude::Pubkey;

use crate::stability_pool::liquidations_queue::EventStatus;
use crate::{CollateralAmounts, LiquidationEvent, LiquidationsQueue};

impl Default for LiquidationsQueue {
    #[cfg(not(test))]
    fn default() -> Self {
        unimplemented!()
    }

    #[cfg(test)]
    #[inline(never)]
    fn default() -> Self {
        use crate::utils::consts::MAX_LIQUIDATION_EVENTS;
        let events: [LiquidationEvent; MAX_LIQUIDATION_EVENTS] =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        LiquidationsQueue { events, len: 0 }
    }
}

impl LiquidationEvent {
    #[inline(never)]
    pub fn empty() -> LiquidationEvent {
        LiquidationEvent {
            status: EventStatus::Inactive as u8,
            user_positions: Pubkey::default(),
            position_index: 0,
            event_ts: 0,
            liquidator: Pubkey::default(),
            collateral_gain_to_liquidator: CollateralAmounts::default(),
            collateral_gain_to_clearer: CollateralAmounts::default(),
            collateral_gain_to_stability_pool: CollateralAmounts::default(),
        }
    }

    pub fn new(
        liquidator: Pubkey,
        liquidator_gains: CollateralAmounts,
        clearer_gains: CollateralAmounts,
        stability_pool_gains: CollateralAmounts,
        event_timestamp: u64,
    ) -> Self {
        LiquidationEvent {
            liquidator,
            collateral_gain_to_liquidator: liquidator_gains,
            collateral_gain_to_clearer: clearer_gains,
            collateral_gain_to_stability_pool: stability_pool_gains,
            event_ts: event_timestamp,
            status: EventStatus::PendingCollection as u8,
            ..Default::default()
        }
    }
}
