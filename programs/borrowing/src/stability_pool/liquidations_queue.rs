#[allow(unused_imports)]
use std::cell::Ref;
use std::cell::RefMut;

use crate::utils::consts::LIQUIDATIONS_SECONDS_TO_CLAIM_GAINS;
use crate::{
    borrowing_market::types::ClearLiquidationGainsEffects, drain_event, state::CollateralToken,
    CollateralAmounts, LiquidationEvent, LiquidationsQueue,
};
use anchor_lang::prelude::Pubkey;

pub enum EventStatus {
    Inactive = 0,
    PendingCollection = 1,
}

pub fn initialize_queue(queue: &mut RefMut<LiquidationsQueue>) {
    queue.len = 0;
}

pub fn add_liquidation_event(
    liquidation_event: LiquidationEvent,
    queue: &mut RefMut<LiquidationsQueue>,
) -> Result<(), crate::BorrowError> {
    let liquidation_index = get_next_index(queue)?;
    queue.len += 1;
    queue.events[liquidation_index as usize] = liquidation_event;
    Ok(())
}

pub fn clear_liquidation_gains(
    queue: &mut RefMut<LiquidationsQueue>,
    token: CollateralToken,
    clearing_agent: Pubkey,
    now_timestamp: u64,
) -> ClearLiquidationGainsEffects {
    // Gets all the outstanding liquidation rewards for a given token
    // If this a run by a liquidator, we are also returning the gains
    // earned by the liquidator from the liquidation event,
    // basically the `TryLiquidate` instruction

    let mut clearing_agent_gains = CollateralAmounts::default();
    let mut stability_pool_gains = CollateralAmounts::default(); // stability_pool_gains

    for i in 0..(*queue).events.len() {
        let mut event: LiquidationEvent = (*queue).events[i as usize];
        if event.status == (EventStatus::PendingCollection as u8) {
            // 1. Drain all the gains pending to the stability pool
            // 2. If the current clearing agent is also the bot that
            // triggered the liquidation, then include the gains
            // for the liquidator also
            let clearing_agent_is_event_liquidator = clearing_agent == event.liquidator;
            match token {
                CollateralToken::SOL => drain_event!(
                    clearing_agent_gains,
                    stability_pool_gains,
                    event,
                    clearing_agent_is_event_liquidator,
                    sol,
                    now_timestamp
                ),
                CollateralToken::ETH => drain_event!(
                    clearing_agent_gains,
                    stability_pool_gains,
                    event,
                    clearing_agent_is_event_liquidator,
                    eth,
                    now_timestamp
                ),
                CollateralToken::BTC => drain_event!(
                    clearing_agent_gains,
                    stability_pool_gains,
                    event,
                    clearing_agent_is_event_liquidator,
                    btc,
                    now_timestamp
                ),
                CollateralToken::FTT => drain_event!(
                    clearing_agent_gains,
                    stability_pool_gains,
                    event,
                    clearing_agent_is_event_liquidator,
                    ftt,
                    now_timestamp
                ),
                CollateralToken::RAY => drain_event!(
                    clearing_agent_gains,
                    stability_pool_gains,
                    event,
                    clearing_agent_is_event_liquidator,
                    ray,
                    now_timestamp
                ),
                CollateralToken::SRM => drain_event!(
                    clearing_agent_gains,
                    stability_pool_gains,
                    event,
                    clearing_agent_is_event_liquidator,
                    srm,
                    now_timestamp
                ),
            }
            queue.events[i as usize] = event;
            if event.collateral_gain_to_liquidator.is_zero()
                && event.collateral_gain_to_stability_pool.is_zero()
            {
                remove_liquidation_event(queue, i);
            }
        }
    }

    ClearLiquidationGainsEffects {
        clearing_agent_gains,
        stability_pool_gains,
    }
}

#[cfg(test)]
pub fn get(queue: &mut RefMut<LiquidationsQueue>, index: usize) -> LiquidationEvent {
    let event: LiquidationEvent = (*queue).events[index];
    event
}

#[cfg(test)]
pub fn len(queue: &mut RefMut<LiquidationsQueue>) -> usize {
    (*queue)
        .events
        .iter()
        .filter(|event| event.status == EventStatus::PendingCollection as u8)
        .count()
}

pub fn get_next_index(queue: &mut RefMut<LiquidationsQueue>) -> Result<usize, crate::BorrowError> {
    // Gets next available index
    // When we move to derived accounts, this will not longer be an issue
    for (i, event) in (*queue).events.iter().enumerate() {
        if event.status == (EventStatus::Inactive as u8) {
            return Ok(i);
        }
    }
    Err(crate::BorrowError::LiquidationsQueueFull)
}

pub fn has_pending_liquidation_events(queue: &mut RefMut<LiquidationsQueue>) -> bool {
    for event in queue.events.iter() {
        // we don't care if the liquidator has not cleared their gains
        // we just care that the stability pool has received all the pending gains
        if event.status == (EventStatus::PendingCollection as u8)
            && !event.collateral_gain_to_stability_pool.is_zero()
        {
            return true;
        }
    }
    false
}

pub fn remove_liquidation_event(queue: &mut RefMut<LiquidationsQueue>, index: usize) {
    let mut event: LiquidationEvent = (*queue).events[index];

    event.status = EventStatus::Inactive as u8;
    queue.len -= 1;
    queue.events[index] = event;
}

mod utils {
    #[macro_export]
    macro_rules! drain_event {
        (
            $clearing_agent_gains:ident,
            $stability_pool_gains: ident,
            $event: ident,
            $clearing_agent_is_event_liquidator: ident,
            $token: ident,
            $now: ident) => {{

            // The liquidation process involves two agesnts: (1) the liquidator and (2) the clearer
            // (they could be the same, but not necessarily)
            // The liquidator is marking users as liquidated. Only performing a state change.
            // The clearer is moving the funds from the collateral_vaults to the liquidation_reward_vaults.

            // The liquidator earns 0.4% of the liquidation amounts and the clearer 0.1% of the gains.
            // The clearer exists such that someone is incentivised to clear this queue and not act
            // maliciously and allow the pool to keep growing. Even if the liquidator is entitled to
            // their gains, they might not run the "clear_liquidations_gains" event and let the queue growing.

            // Therefore we allow the clearer to move part of the funds, however the liquidator has to
            // move their funds as well. If they don't do it, the queue can get full. We might just
            // debit their account by keeping a handle to their collateral ata, but seems unnecessary.

            // To incentivise the liquidator and the clearer to run these transactions, we give a 5 seconds window
            // to the liquidator to clear their gains, after that, anyone that runs this transction is entitled to them.

            $stability_pool_gains.$token += $event.collateral_gain_to_stability_pool.$token;
            $event.collateral_gain_to_stability_pool.$token = 0;

            // What belongs to the clearer no matter what
            $clearing_agent_gains.$token += $event.collateral_gain_to_clearer.$token;
            $event.collateral_gain_to_clearer.$token = 0;

            // What belongs to the liquidator
            if $clearing_agent_is_event_liquidator {
                $clearing_agent_gains.$token += $event.collateral_gain_to_liquidator.$token;
                $event.collateral_gain_to_liquidator.$token = 0;
            }

            // What belongs to the clearer if the liquidator is lazy
            if $event.event_ts + LIQUIDATIONS_SECONDS_TO_CLAIM_GAINS < $now {
                $clearing_agent_gains.$token += $event.collateral_gain_to_liquidator.$token;
                $event.collateral_gain_to_liquidator.$token = 0;
            }
        }};
    }
}
