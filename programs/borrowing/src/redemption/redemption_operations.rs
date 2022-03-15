use crate::{
    borrowing_market::borrowing_rate::{self, FeeEvent},
    state::{
        redemptions_queue::{RedemptionCandidateStatus, RedemptionOrderStatus},
        BorrowingMarketState, CandidateRedemptionUser, RedemptionOrder, RedemptionsQueue,
    },
    utils::consts::NORMAL_MCR,
    utils::{coretypes::CheckedAssign, finance::CollateralInfo},
};

use crate::redemption::types::RedemptionCollateralSplit;
use anchor_lang::prelude::msg;
#[cfg(not(test))]
use anchor_lang::solana_program::log::sol_log_compute_units;

// Adding an order
// When a user asks to redeem x amount of USDH, we take the following actions:
// 1. We place an order on the RedemptionsQueue.
// 2. The Redemptions queue invites bots (fillers) to submit users with lowest CR
// 3. We give 5 seconds for fillers to submitted users
// 4. After that we start clearing them off the queue
// 5. Once clearing is over, we pop the redemption order off the queue.

// Filling an order
// Once the RedemptionOrder is added, its status is set to Filling and
// fillers can start submitting candidates for redemption. This
// instruction sorts and merges them in chunks, preparing the
// users to be redeemed for the 'clear_order' instruction

// Clearing an order
// When the order is eligible for Clearing (has been filled for more than 5 seconds)
// then clearers are allowed to submit the users that can be redeemed against.
// The clearers will submit an array of &[&mut UserMetadata], limited to 32 max
// these are the only accounts we can modify on this execution due to the solana limit
// they will be a permutation of User accounts and Filler accounts.
// For example, if user has $120 in collateral and $100 debt
// we are only redeeming 100 of debt (equivalent in collateral)
// therefore we are claiming a ratio of 100/120 = 0.83 off his collateral

use crate::log_compute_units;
use crate::{
    redemption::types::ClearRedemptionOrderEffects, utils::consts::MIN_REDEMPTIONS_AMOUNT_USDH,
    BorrowError::*, TokenPrices, UserMetadata,
};

use crate::redemption::types::AddRedemptionOrderEffects;
use std::cell::RefMut;

use super::types::RedemptionFillingResults;

pub fn add_redemption_order(
    redeemer: &mut UserMetadata,
    queue: &mut RefMut<RedemptionsQueue>,
    market: &mut BorrowingMarketState,
    prices: &TokenPrices,
    now_timestamp: u64,
    redemption_amount: u64,
) -> Result<AddRedemptionOrderEffects, crate::BorrowError> {
    if redemption_amount < MIN_REDEMPTIONS_AMOUNT_USDH {
        return Err(RedemptionsAmountTooSmall);
    }

    let outstanding_redemptions = queue::calculate_outstanding_redemption_amount(queue);
    let remaining_supply = market.stablecoin_borrowed - outstanding_redemptions;
    if redemption_amount > remaining_supply {
        return Err(CannotRedeemMoreThanMinted);
    }

    if now_timestamp < market.bootstrap_period_timestamp {
        return Err(CannotRedeemDuringBootstrapPeriod);
    }

    let tcr = CollateralInfo::calc_coll_ratio(
        market.stablecoin_borrowed,
        &market.deposited_collateral,
        prices,
    );

    if tcr.to_percent()? < NORMAL_MCR as u128 {
        return Err(CannotRedeemWhenUndercollateralized);
    }

    borrowing_rate::refresh_base_rate(
        market,
        FeeEvent::Redemption {
            redeeming: redemption_amount,
            supply: remaining_supply,
        },
        now_timestamp,
    )?;

    let order = queue::add_redemption_order(
        redemption_amount,
        queue,
        redeemer,
        prices,
        now_timestamp,
        market.base_rate_bps,
    )?;

    Ok(AddRedemptionOrderEffects {
        redemption_order_id: order.id,
        transfer_stablecoin_amount: order.requested_amount,
    })
}

pub fn fill_redemption_order(
    order_id: u64,
    market: &mut BorrowingMarketState,
    queue: &mut RefMut<RedemptionsQueue>,
    user_metadatas: &mut [&mut UserMetadata],
    filler_metadata: &UserMetadata,
    now_timestamp: u64,
) -> Result<(), crate::BorrowError> {
    if user_metadatas.is_empty() {
        return Ok(());
    }

    let order = queue::next_fill_order(queue, order_id, now_timestamp)?;

    log_compute_units!("Before process users.");
    let candidates =
        sort::extract_transform_sort_candidates(market, order, user_metadatas, filler_metadata)?;

    // Merge new with existing, prioritizing existing if smaller or equal
    log_compute_units!("Before merge users.");
    sort::merge(candidates, &mut order.candidate_users);

    Ok(())
}

pub fn clear_redemption_order<'a, 'b>(
    order_id: u64,
    redeemer: &'a mut UserMetadata,
    clearer: &'a mut UserMetadata,
    market: &'a mut BorrowingMarketState,
    redemptions_queue: &'a mut RefMut<RedemptionsQueue>,
    fillers_and_borrowers: &'a mut [&'b mut UserMetadata],
    now_timestamp: u64,
) -> Result<ClearRedemptionOrderEffects, crate::BorrowError> {
    sort::assert_unique(fillers_and_borrowers)?;

    let order = queue::next_clear_order(
        redemptions_queue,
        order_id,
        &redeemer.metadata_pk,
        now_timestamp,
    )?;

    let RedemptionFillingResults {
        collateral_redeemed,
        collateral_made_inactive,
        debt_redeemed,
    } = queue::collect_collateral_and_pay_debt(market, order, fillers_and_borrowers)?;

    // Reward redeemer and cleared
    redeemer
        .inactive_collateral
        .add_assign(&collateral_redeemed.redeemer);

    clearer
        .inactive_collateral
        .add_assign(&collateral_redeemed.clearer);

    // Turn the global collateral to inactive
    let collateral_made_inactive = collateral_made_inactive.add(&collateral_redeemed.total);
    market
        .deposited_collateral
        .sub_assign(&collateral_made_inactive);

    market
        .inactive_collateral
        .add_assign(&collateral_made_inactive);

    // Remove debt
    market
        .stablecoin_borrowed
        .checked_sub_assign(debt_redeemed)
        .unwrap();

    queue::flush_order(order);

    Ok(ClearRedemptionOrderEffects {
        redeemed_stablecoin: debt_redeemed,
        redeemed_collateral: collateral_redeemed,
    })
}

mod sort {
    use crate::{
        log_compute_units,
        redemption::redemption_operations::calcs,
        state::{BorrowingMarketState, CandidateRedemptionUser, RedemptionOrder},
        BorrowError, UserMetadata,
    };
    use anchor_lang::prelude::msg;
    #[cfg(not(test))]
    use anchor_lang::solana_program::log::sol_log_compute_units;

    pub fn extract_transform_sort_candidates(
        market: &mut BorrowingMarketState,
        redemption_order: &RedemptionOrder,
        candidates: &mut [&mut UserMetadata],
        filler_metadata: &UserMetadata,
    ) -> Result<Vec<CandidateRedemptionUser>, BorrowError> {
        // Clean up, sort, dedup, get MV for submitted users
        // We don't trust any of the data coming from off-chain
        // So we need to recalculate all the data
        // For each user: sort, uniq, calculate_market_value

        assert_unique(candidates)?;
        let mut cleaned_candidates = Vec::with_capacity(candidates.len());

        for user_metadata in candidates.iter_mut() {
            if let Ok(Some(res)) = calcs::calculate_candidate(
                market,
                user_metadata,
                &redemption_order.redemption_prices,
                filler_metadata.metadata_pk,
            ) {
                cleaned_candidates.push(res);
            }
        }

        log_compute_units!("Before sort candidates.");
        cleaned_candidates.sort_by(|left, right| {
            left.collateral_ratio
                .partial_cmp(&right.collateral_ratio)
                .unwrap()
        });

        Ok(cleaned_candidates)
    }

    pub fn merge(
        new_candidates: Vec<CandidateRedemptionUser>,
        current_candidates: &mut [CandidateRedemptionUser; 32],
    ) {
        let (mut i, mut j) = (0, 0);
        while i < new_candidates.len() && j < current_candidates.len() {
            let existing_position = current_candidates[j];
            let candidate_position = new_candidates[i];
            if existing_position.status == 0 {
                let mut k = current_candidates.len() - 1;
                while k > j {
                    current_candidates[k] = current_candidates[k - 1];
                    k -= 1;
                }
                current_candidates[j] = candidate_position;
                i += 1;
                j += 1;
                continue;
            }
            if existing_position.user_id == candidate_position.user_id {
                i += 1;
                j += 1;
                continue;
            }
            if existing_position.collateral_ratio <= candidate_position.collateral_ratio {
                j += 1;
                continue;
            } else {
                let mut k = current_candidates.len() - 1;
                while k > j {
                    current_candidates[k] = current_candidates[k - 1];
                    k -= 1;
                }
                current_candidates[j] = candidate_position;
                i += 1;
                j += 1;
                continue;
            }
        }
    }

    pub fn assert_unique(
        fillers_and_borrowers: &mut [&mut UserMetadata],
    ) -> Result<(), crate::BorrowError> {
        let mut user_ids: Vec<u64> = fillers_and_borrowers.iter().map(|u| u.user_id).collect();
        user_ids.sort_unstable();
        user_ids.dedup();

        if user_ids.len() < fillers_and_borrowers.len() {
            return Err(crate::BorrowError::DuplicateAccountInFillOrder);
        }

        Ok(())
    }
}

pub mod calcs {

    use super::RedemptionCollateralSplit;
    use crate::borrowing_market::borrowing_operations::redistribution::update_user_stake_and_total_stakes;
    use crate::BorrowError;
    use crate::CollateralAmounts;
    use crate::{borrowing_market::borrowing_rate, utils::consts::*};

    use anchor_lang::prelude::{msg, Pubkey};

    use crate::{
        borrowing_market::borrowing_operations::apply_pending_rewards,
        state::{CandidateRedemptionUser, UserStatus},
        utils::finance::CollateralInfo,
        BorrowingMarketState, RedemptionOrder, TokenPrices, UserMetadata,
    };

    pub fn split_redemption_collateral(
        total: &CollateralAmounts,
        base_rate_bps: u16,
    ) -> RedemptionCollateralSplit {
        let one = 10_000; // bps
        let redemption_fee = borrowing_rate::calc_redemption_fee(base_rate_bps);

        // 100.0%
        //   0.4% to stakers
        //   0.1% to filler & clearers
        // given a base_rate of 0% (redemption rate of 0.5%)

        // msg!(
        //     "BaseRate {}, RedemptionRate {}",
        //     base_rate_bps,
        //     redemption_fee
        // );

        let mut redeemer = total.mul_bps(one - redemption_fee);

        // 10 bps
        let filler = total.mul_bps(REDEMPTION_FILLER);
        let clearer = total.mul_bps(REDEMPTION_CLEARER);

        // rest goes to stakers
        // 40 bps normally, but could be higher
        let stakers_bps = redemption_fee - REDEMPTION_FILLER - REDEMPTION_CLEARER;
        let stakers = total.mul_bps(stakers_bps);

        // println!("Redeeming from {:?}", total);
        // println!("Redeeming redeemer {:?}", redeemer);
        // println!("Redeeming stakers {:?}", stakers);
        // println!("Redeeming filler {:?}", filler);
        // println!("Redeeming clearer {:?}", clearer);

        let remaining = total
            .sub(&redeemer)
            .sub(&stakers)
            .sub(&filler)
            .sub(&clearer);

        redeemer.add_assign(&remaining);

        RedemptionCollateralSplit {
            filler,
            clearer,
            redeemer,
            stakers,
            total: *total,
        }
    }
    pub fn calculate_candidate(
        market: &mut BorrowingMarketState,
        user_metadata: &mut UserMetadata,
        prices: &TokenPrices,
        filler_metadata: Pubkey,
    ) -> Result<Option<CandidateRedemptionUser>, BorrowError> {
        if user_metadata.status == (UserStatus::Active as u8) {
            apply_pending_rewards(market, user_metadata)?;
            update_user_stake_and_total_stakes(market, user_metadata);
            if user_metadata.borrowed_stablecoin == 0 {
                return Ok(None);
            }
            let CollateralInfo {
                collateral_ratio, ..
            } = CollateralInfo::from(user_metadata, prices);
            // println!("Adding user with CR {}%", collateral_ratio.to_percent()?);
            if collateral_ratio.to_percent()? < NORMAL_MCR as u128 {
                return Ok(None);
            }

            return Ok(Some(CandidateRedemptionUser {
                status: 1,
                user_id: user_metadata.user_id,
                debt: user_metadata.borrowed_stablecoin,
                collateral_ratio: collateral_ratio.try_floor_u64().unwrap(),
                filler_metadata,
                user_metadata: user_metadata.metadata_pk,
            }));
        }

        Ok(None)
    }

    pub fn calc_redemption_amounts(
        fillers_and_borrowers: &mut [&mut UserMetadata],
        redemption_order: &RedemptionOrder,
        user_to_redeem_ix: usize,
        candidate_user_ix: usize,
        remaining_amount: u64,
    ) -> Option<(u64, CollateralAmounts)> {
        let borrowed = fillers_and_borrowers[user_to_redeem_ix].borrowed_stablecoin;
        let amount_to_redeem = u64::min(remaining_amount, borrowed);
        let collateral_info = CollateralInfo::from(
            fillers_and_borrowers[user_to_redeem_ix],
            &redemption_order.redemption_prices,
        );

        if collateral_info.collateral_ratio.try_floor_u64().unwrap()
            != redemption_order.candidate_users[candidate_user_ix].collateral_ratio
        {
            // The user has changed their CR since the 'fill' event,
            // skip it
            msg!(
                "User {} has changed since fill, skipping.",
                candidate_user_ix
            );
            // continue 'candidates_loop;
            return None;
        }

        // ratios is how much of the collateral is the redemed amount worth
        let mv = collateral_info.collateral_value;
        let collateral_to_redeem = fillers_and_borrowers[user_to_redeem_ix]
            .deposited_collateral
            .mul_fraction(amount_to_redeem, mv);
        // println!(
        //     "User has debt USDH={}, collateral USDH={}, CR={:?}, having collaterals {:?}, redeeming {:?}",
        //     fillers_and_borrowers[user_to_redeem_ix].borrowed_stablecoin,
        //     collateral_info.collateral_value,
        //     collateral_info.collateral_ratio,
        //     fillers_and_borrowers[user_to_redeem_ix].deposited_collateral,
        //     collateral_to_redeem
        // );

        Some((amount_to_redeem, collateral_to_redeem))
    }
}

mod queue {
    use anchor_lang::prelude::Pubkey;

    use crate::{
        borrowing_market::borrowing_operations::redistribution,
        fail, some_or_continue,
        utils::{consts::REDEMPTIONS_SECONDS_TO_FILL_ORDER, coretypes::CheckedAssign},
        BorrowError, CollateralAmounts, TokenPrices, UserMetadata,
    };

    use super::*;
    use std::cell::RefMut;

    pub fn next_fill_order<'a>(
        redemptions_queue: &'a mut RefMut<RedemptionsQueue>,
        order_id: u64,
        now: u64,
    ) -> Result<&'a mut RedemptionOrder, BorrowError> {
        match first_outstanding(redemptions_queue) {
            None => {
                fail!(BorrowError::RedemptionsQueueIsEmpty);
            }
            Some((index, id)) => {
                if id != order_id {
                    // this is not really important tbh
                    // we should always just fill the first next available
                    // fail!(BorrowError::InvalidRedemptionOrder);
                }
                let order = &mut redemptions_queue.orders[index];
                let status = RedemptionOrderStatus::from(order.status);
                match status {
                    RedemptionOrderStatus::Open => {
                        order.status = RedemptionOrderStatus::Filling.into();
                        order.last_reset = now;
                    }
                    RedemptionOrderStatus::Filling => {}
                    RedemptionOrderStatus::Claiming => {
                        fail!(BorrowError::CannotFillRedemptionOrderWhileInClearingMode);
                    }
                    RedemptionOrderStatus::Inactive => {
                        fail!(BorrowError::InvalidRedemptionOrder);
                    }
                }
                Ok(order)
            }
        }
    }

    pub fn next_clear_order<'a>(
        redemptions_queue: &'a mut RefMut<RedemptionsQueue>,
        order_id: u64,
        redeemer_metadata: &Pubkey,
        now: u64,
    ) -> Result<&'a mut RedemptionOrder, BorrowError> {
        match first_outstanding(redemptions_queue) {
            None => {
                fail!(BorrowError::RedemptionsQueueIsEmpty);
            }
            Some((index, id)) => {
                if id != order_id {
                    // this is not really important tbh
                    // we should always just clear the first next available
                    // fail!(BorrowError::InvalidRedemptionOrder);
                }
                let order = &mut redemptions_queue.orders[index];
                if &order.redeemer_user_metadata != redeemer_metadata {
                    fail!(BorrowError::InvalidRedeemer);
                }
                let status: RedemptionOrderStatus = order.status.into();

                match status {
                    RedemptionOrderStatus::Open => {
                        fail!(BorrowError::CannotClearRedemptionOrderWhileInFillingMode);
                    }
                    RedemptionOrderStatus::Filling => {
                        if order.last_reset + REDEMPTIONS_SECONDS_TO_FILL_ORDER > now {
                            fail!(BorrowError::CannotClearRedemptionOrderWhileInFillingMode);
                        } else {
                            order.status = RedemptionOrderStatus::Claiming.into();
                        }
                    }
                    RedemptionOrderStatus::Claiming => {}
                    RedemptionOrderStatus::Inactive => {
                        fail!(BorrowError::InvalidRedemptionOrder);
                    }
                };
                Ok(order)
            }
        }
    }

    pub fn first_outstanding(
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
    ) -> Option<(usize, u64)> {
        redemptions_queue
            .orders
            .iter()
            .enumerate()
            .filter(|(_index, o)| o.status != RedemptionOrderStatus::Inactive as u8)
            .min_by_key(|(_index, o)| o.id)
            .map(|(index, o)| (index, o.id))
    }

    pub fn add_redemption_order<'a, 'b>(
        amount: u64,
        queue: &'a mut RefMut<RedemptionsQueue>,
        redeemer: &'b UserMetadata,
        prices: &'b TokenPrices,
        now: u64,
        base_rate: u16,
    ) -> Result<&'a RedemptionOrder, BorrowError> {
        // Zeroes out the next available order on the queue and updates it with
        // relevant data such as requested amount, requester, etc

        let index = queue
            .orders
            .iter()
            .position(|o| o.status == RedemptionOrderStatus::Inactive as u8)
            .ok_or(BorrowError::RedemptionsQueueIsFull)?;

        let next_index = queue.next_index;
        queue.next_index += 1;

        let mut order: &mut RedemptionOrder = &mut queue.orders[index];
        order.id = next_index;
        order.status = RedemptionOrderStatus::Open as u8;
        order.base_rate = base_rate;
        order.redeemer = redeemer.owner;
        order.requested_amount = amount;
        order.remaining_amount = amount;
        order.last_reset = now;
        order.redeemer_user_metadata = redeemer.metadata_pk;
        order.redemption_prices = *prices;

        for i in 0..order.candidate_users.len() {
            order.candidate_users[i].status = 0;
        }

        // queue.orders[index] = order;

        Ok(order)
    }

    pub fn collect_collateral_and_pay_debt(
        market: &mut BorrowingMarketState,
        order: &mut RedemptionOrder,
        fillers_and_borrowers: &mut [&mut UserMetadata],
    ) -> Result<RedemptionFillingResults, crate::BorrowError> {
        let mut total_collateral_gains = RedemptionCollateralSplit::default();
        let mut total_collateral_made_inactive = CollateralAmounts::default();
        let mut remaining_amount = order.remaining_amount;
        let mut claimed_amount = 0;

        'candidates_loop: for i in 0..order.candidate_users.len() {
            log_compute_units!("Looping through user {}", i);
            if remaining_amount == 0 {
                break 'candidates_loop;
            }

            if order.candidate_users[i].status != RedemptionCandidateStatus::Active as u8 {
                // This is an invalid entry (or end of list)
                // nothing to do here, we should stop
                break 'candidates_loop;
            }

            let (user, filler) = match queue::map_accounts_to_candidate_user(
                &i,
                &order.candidate_users,
                fillers_and_borrowers,
            ) {
                Ok(v) => v,
                Err(crate::BorrowError::RedemptionFillerNotFound) => {
                    return Err(RedemptionFillerNotFound);
                }
                Err(crate::BorrowError::RedemptionUserNotFound) => {
                    // ensure that the user_to_redeem and filler are both found
                    // else error, we should disallow clearing in any other order
                    // other than the currently sorted one
                    // if we have not found the next correct user, we break out of the loop
                    msg!(
                        "Could not find user for candidate {:?} {:?}. Stopping early.",
                        &order.candidate_users[i].user_metadata,
                        RedemptionUserNotFound
                    );
                    break 'candidates_loop;
                }
                _ => return Err(WrongRedemptionUser),
            };

            let (redeemed_amount, redeemed_collateral) = some_or_continue!(calcs::calc_redemption_amounts(
                    fillers_and_borrowers,
                    order,
                    user,
                    i,
                    remaining_amount
                ), 'candidates_loop);

            let collateral_split =
                calcs::split_redemption_collateral(&redeemed_collateral, order.base_rate);

            claimed_amount += redeemed_amount;
            remaining_amount = remaining_amount.checked_sub(redeemed_amount).unwrap();
            total_collateral_gains.checked_add_assign(&collateral_split);

            fillers_and_borrowers[filler]
                .inactive_collateral
                .add_assign(&collateral_split.filler);

            fillers_and_borrowers[user]
                .deposited_collateral
                .sub_assign(&redeemed_collateral);

            fillers_and_borrowers[user]
                .borrowed_stablecoin
                .checked_sub_assign(redeemed_amount)?;

            redistribution::update_user_stake_and_total_stakes(market, fillers_and_borrowers[user]);

            if fillers_and_borrowers[user].borrowed_stablecoin == 0 {
                // Mark for cleaning, no longer useful for this order
                order.candidate_users[i].status = RedemptionOrderStatus::Inactive.into();

                // Full redemption, no more debt, no need for the collateral to be active
                // Also, mark the collateral as inactive since it's no longer backing and USDH debt
                // and it would be misleadingly indicate the system is overcollateralized,
                total_collateral_made_inactive
                    .add_assign(&fillers_and_borrowers[user].deposited_collateral);

                fillers_and_borrowers[user]
                    .inactive_collateral
                    .add_assign(&fillers_and_borrowers[user].deposited_collateral);

                fillers_and_borrowers[user].deposited_collateral = CollateralAmounts::default();

                market.num_active_users -= 1;
            }
        }

        order.remaining_amount = remaining_amount;

        Ok(RedemptionFillingResults {
            collateral_redeemed: total_collateral_gains,
            collateral_made_inactive: total_collateral_made_inactive,
            debt_redeemed: claimed_amount,
        })
    }

    pub fn flush_order(redemption_order: &mut RedemptionOrder) {
        // Update the queue:
        // - if the redemption has been fully completed,
        // there is nothing left to be redeemed,
        // so we close out the entire order
        // - else pop the redeemed candidates off the queue
        // we need to leave the queue in a clean spot
        if redemption_order.remaining_amount == 0 {
            msg!("Redemption order filled, removing order.");
            queue::close_redemption_order(redemption_order);
        } else {
            msg!("Redemption order partially filled, popping redeemed users.");
            queue::refresh_unfulfilled_order(redemption_order);
        }
    }

    pub fn close_redemption_order(order: &mut RedemptionOrder) {
        order.id = 0;
        order.status = RedemptionOrderStatus::Inactive as u8;
        order.redeemer = Pubkey::default();
        order.requested_amount = 0;
        order.remaining_amount = 0;
        order.last_reset = 0;
        order.redeemer_user_metadata = Pubkey::default();
        order.redemption_prices = TokenPrices::default();

        for i in 0..order.candidate_users.len() {
            order.candidate_users[i].status = 0;
        }
    }

    pub fn map_accounts_to_candidate_user<'a>(
        index: &usize,
        candidates: &'a [CandidateRedemptionUser],
        fillers_and_borrowers: &[&'a mut UserMetadata],
    ) -> Result<(usize, usize), BorrowError> {
        let candidate_user_to_redeem = candidates[*index];
        let user_to_redeem_idx = fillers_and_borrowers
            .iter()
            .enumerate()
            .find(|(_i, user)| user.metadata_pk == candidate_user_to_redeem.user_metadata)
            .ok_or(BorrowError::RedemptionUserNotFound)?
            .0;

        let filler_to_reward_idx = fillers_and_borrowers
            .iter()
            .enumerate()
            .find(|(_i, user)| user.metadata_pk == candidate_user_to_redeem.filler_metadata)
            .ok_or(BorrowError::RedemptionFillerNotFound)?
            .0;

        Ok((user_to_redeem_idx, filler_to_reward_idx))
    }

    pub fn refresh_unfulfilled_order(redemption_order: &mut RedemptionOrder) {
        let valid_users: Vec<CandidateRedemptionUser> = redemption_order
            .candidate_users
            .iter()
            .filter(|user| user.status == RedemptionCandidateStatus::Active as u8)
            .copied()
            .collect();

        if valid_users.is_empty() {
            // order is still unfulfilled, but there are no more valid users
            // therefore we set the order back to Open mode
            redemption_order.status = RedemptionOrderStatus::Open.into();
        } else {
            redemption_order
                .candidate_users
                .iter_mut()
                .enumerate()
                .for_each(|(i, user)| {
                    if i < valid_users.len() {
                        *user = valid_users[i];
                    } else {
                        user.status = RedemptionCandidateStatus::Inactive.into();
                    }
                });
        }
    }

    pub fn calculate_outstanding_redemption_amount(queue: &mut RefMut<RedemptionsQueue>) -> u64 {
        queue
            .orders
            .iter()
            .map(|o| {
                if o.status != RedemptionOrderStatus::Inactive as u8 {
                    o.remaining_amount
                } else {
                    0
                }
            })
            .sum()
    }
}
