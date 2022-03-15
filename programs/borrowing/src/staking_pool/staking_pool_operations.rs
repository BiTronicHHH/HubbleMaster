use super::types::{HarvestEffects, UnstakeEffects};
use crate::{utils::consts::DECIMAL_PRECISION, BorrowError, StakingPoolState, UserStakingState};
use anchor_lang::prelude::ProgramError;

pub fn initialize_staking_pool(staking_pool_state: &mut StakingPoolState) {
    staking_pool_state.initialize_staking_pool();
}

pub fn approve_new_user(
    staking_pool_state: &mut StakingPoolState,
    user_staking_state: &mut UserStakingState,
) -> Result<(), BorrowError> {
    let user_id = staking_pool_state.num_users;

    user_staking_state.version = 0;
    user_staking_state.user_id = user_id;

    user_staking_state.rewards_tally = 0;
    user_staking_state.user_stake = 0;

    // Update global state
    staking_pool_state.num_users = staking_pool_state.num_users.checked_add(1).unwrap();

    Ok(())
}

pub fn user_stake(
    staking_pool_state: &mut StakingPoolState,
    user_staking_state: &mut UserStakingState,
    amount: u64,
) {
    user_staking_state.user_stake = user_staking_state
        .user_stake
        .checked_add(amount as u128)
        .unwrap();

    user_staking_state.rewards_tally = user_staking_state
        .rewards_tally
        .checked_add(
            (amount as u128)
                .checked_mul(staking_pool_state.reward_per_token)
                .unwrap(),
        )
        .unwrap();

    staking_pool_state.total_stake = staking_pool_state
        .total_stake
        .checked_add(amount as u128)
        .unwrap();
}

pub fn user_harvest(
    staking_pool_state: &mut StakingPoolState,
    user_staking_state: &mut UserStakingState,
) -> Result<HarvestEffects, ProgramError> {
    // Unscale the reward
    let reward = user_staking_state
        .user_stake
        .checked_mul(staking_pool_state.reward_per_token)
        .unwrap()
        .checked_sub(user_staking_state.rewards_tally)
        .unwrap();
    let reward = reward.checked_div(DECIMAL_PRECISION).unwrap();

    user_staking_state.rewards_tally = user_staking_state
        .user_stake
        .checked_mul(staking_pool_state.reward_per_token)
        .unwrap();

    staking_pool_state.rewards_not_yet_claimed = staking_pool_state
        .rewards_not_yet_claimed
        .checked_sub(reward)
        .unwrap();

    Ok(HarvestEffects { reward })
}

pub fn user_unstake(
    staking_pool_state: &mut StakingPoolState,
    user_staking_state: &mut UserStakingState,
    amount: u64,
) -> Result<UnstakeEffects, ProgramError> {
    // Unscale the reward
    let reward = user_staking_state
        .user_stake
        .checked_mul(staking_pool_state.reward_per_token)
        .unwrap()
        .checked_sub(user_staking_state.rewards_tally)
        .unwrap();
    let reward = reward.checked_div(DECIMAL_PRECISION).unwrap();

    let amount_to_withdraw = std::cmp::min(amount as u128, user_staking_state.user_stake);

    user_staking_state.rewards_tally = user_staking_state
        .user_stake
        .checked_mul(staking_pool_state.reward_per_token)
        .unwrap();

    staking_pool_state.rewards_not_yet_claimed = staking_pool_state
        .rewards_not_yet_claimed
        .checked_sub(reward)
        .unwrap();

    user_staking_state.user_stake = user_staking_state
        .user_stake
        .checked_sub(amount_to_withdraw)
        .unwrap();

    user_staking_state.rewards_tally = user_staking_state
        .rewards_tally
        .checked_sub(
            amount_to_withdraw
                .checked_mul(staking_pool_state.reward_per_token)
                .unwrap(),
        )
        .unwrap();

    staking_pool_state.total_stake = staking_pool_state
        .total_stake
        .checked_sub(amount_to_withdraw)
        .unwrap();

    Ok(UnstakeEffects {
        reward,
        amount_to_withdraw: amount_to_withdraw as u64,
    })
}

pub fn split_fees(fees_to_pay: u64, treasury_fee_rate: u16) -> (u64, u64) {
    let treasury_fee = fees_to_pay * (treasury_fee_rate as u64) / 10_000;
    let staking_fee = fees_to_pay.checked_sub(treasury_fee).unwrap();

    (staking_fee, treasury_fee)
}

pub fn distribute_fees(staking_pool_state: &mut StakingPoolState, fees_to_pay: u64) {
    staking_pool_state.total_distributed_rewards = staking_pool_state
        .total_distributed_rewards
        .checked_add(fees_to_pay as u128)
        .unwrap();

    staking_pool_state.rewards_not_yet_claimed = staking_pool_state
        .rewards_not_yet_claimed
        .checked_add(fees_to_pay as u128)
        .unwrap();

    // scale the reward
    let extra_reward_scaled = (fees_to_pay as u128)
        .checked_mul(DECIMAL_PRECISION)
        .unwrap()
        .checked_add(staking_pool_state.prev_reward_loss)
        .unwrap();

    if staking_pool_state.total_stake != 0 {
        let extra_reward_per_token = extra_reward_scaled
            .checked_div(staking_pool_state.total_stake)
            .unwrap();

        let reward_loss = extra_reward_scaled
            .checked_sub(
                extra_reward_per_token
                    .checked_mul(staking_pool_state.total_stake)
                    .unwrap(),
            )
            .unwrap();

        // println!(
        //     "Adding extra reward {:?} loss {}",
        //     extra_reward_scaled, reward_loss
        // );

        staking_pool_state.reward_per_token = staking_pool_state
            .reward_per_token
            .checked_add(extra_reward_per_token)
            .unwrap();

        staking_pool_state.prev_reward_loss = reward_loss;
    }
}
