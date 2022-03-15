#[cfg(test)]
mod tests {

    use crate::{
        borrowing_market::{borrowing_operations, borrowing_rate::BorrowSplit},
        staking_pool::{
            staking_pool_operations,
            tests::utils,
            types::{HarvestEffects, UnstakeEffects},
        },
        utils::{
            consts::DECIMAL_PRECISION,
            coretypes::{HBB, SOL, USDH},
        },
        BorrowingMarketState, StakingPoolState, UserStakingState,
    };

    #[test]
    fn test_staking_initialize() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        assert_eq!(staking_pool_state.reward_per_token, 0);
        assert_eq!(staking_pool_state.total_stake, 0);
        assert_eq!(staking_pool_state.rewards_not_yet_claimed, 0);
        assert_eq!(staking_pool_state.total_distributed_rewards, 0);
    }

    #[test]
    fn test_staking_approve_vault() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut user = UserStakingState::default();

        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);
        staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();

        assert_eq!(user.user_id, 0);
        assert_eq!(user.version, 0);

        assert_eq!(user.user_stake, 0);
        assert_eq!(user.rewards_tally, 0);
    }

    #[test]
    fn test_staking_approve_vault_multi() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        for i in 0..100 {
            let mut user = UserStakingState::default();

            staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();

            assert_eq!(user.user_id, i);
            assert_eq!(user.version, 0);

            assert_eq!(user.user_stake, 0);
            assert_eq!(user.rewards_tally, 0);
        }
    }

    #[test]
    fn test_staking_stake() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut user = UserStakingState::default();

        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);
        staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();

        let hbb_deposited = HBB::from(100.0);

        staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited);

        println!("user {:}", user.to_state_string());

        assert_eq!(user.user_stake, hbb_deposited as u128);
        assert_eq!(user.rewards_tally, 0);

        assert_eq!(staking_pool_state.total_stake, hbb_deposited as u128);
    }

    #[test]
    fn test_staking_stake_multiple() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let hbb_deposited = HBB::from(100.0);
        let count = 100;

        for _ in 0..count {
            let mut user = UserStakingState::default();

            staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();

            staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited);

            assert_eq!(user.user_stake, hbb_deposited as u128);
            assert_eq!(user.rewards_tally, 0);
        }

        assert_eq!(
            staking_pool_state.total_stake,
            (hbb_deposited * count) as u128
        );
    }

    #[test]
    fn test_staking_unstake() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut user = UserStakingState::default();

        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);
        staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();

        let hbb_deposited = HBB::from(100.0);

        staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited);
        staking_pool_operations::user_unstake(&mut staking_pool_state, &mut user, hbb_deposited)
            .unwrap();

        println!("user {:}", user.to_state_string());
        println!("staking pool {:}", staking_pool_state.to_state_string());

        assert_eq!(user.user_stake, 0);
        assert_eq!(user.rewards_tally, 0);

        assert_eq!(staking_pool_state.total_stake, 0);
    }

    #[test]
    fn test_staking_unstake_multiple() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let hbb_deposited = HBB::from(100.0);
        let count = 100;

        for _ in 0..count {
            let mut user = UserStakingState::default();

            staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();

            staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited);
            staking_pool_operations::user_unstake(
                &mut staking_pool_state,
                &mut user,
                hbb_deposited,
            )
            .unwrap();

            assert_eq!(user.user_stake, 0);
            assert_eq!(user.rewards_tally, 0);
        }

        assert_eq!(staking_pool_state.total_stake, 0);
    }

    #[test]
    fn test_staking_harvest_single() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut market = BorrowingMarketState::default();
        let mut user = UserStakingState::default();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);
        staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();

        let hbb_deposited = HBB::from(100.0);
        let deposit_collateral = SOL::from(15.0);
        let amount_to_borrow = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let now_timestamp = 0;

        println!("borrow_split {:?}", borrow_split);
        staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited);

        utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            1,
            borrow_split.clone(),
            deposit_collateral,
            now_timestamp,
        );

        let HarvestEffects { reward } =
            staking_pool_operations::user_harvest(&mut staking_pool_state, &mut user).unwrap();

        println!("user {:}", user.to_state_string());
        println!("staking pool {:}", staking_pool_state.to_state_string());

        let fees_to_pay = borrow_split.fees_to_pay as u128;
        let treasury_fee = fees_to_pay * 1_500 / 10_000;
        let staking_fee = fees_to_pay - treasury_fee;
        assert_eq!(reward, staking_fee);

        assert_eq!(user.user_stake, hbb_deposited as u128);
        assert_eq!(user.rewards_tally, staking_fee * DECIMAL_PRECISION);

        assert_eq!(staking_pool_state.total_stake, hbb_deposited as u128);
        assert_eq!(staking_pool_state.rewards_not_yet_claimed, 0);
        assert_eq!(staking_pool_state.total_distributed_rewards, staking_fee);
        assert_eq!(
            staking_pool_state.reward_per_token,
            staking_fee * DECIMAL_PRECISION / staking_pool_state.total_stake,
        );
    }

    #[test]
    fn test_staking_harvest_multiple() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut market = BorrowingMarketState::default();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let count = 100;
        let amount_to_borrow = USDH::from(200.0);
        let hbb_deposited = HBB::from(100.0);
        let deposit_collateral = SOL::from(15.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let now_timestamp = 0;

        let mut intermediary_reward_token: u128 = 0;
        let mut prev_loss = 0;

        for i in 0..count {
            println!("Count {} of {}", i, count);
            let mut user = UserStakingState::default();
            staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();

            staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited);

            utils::new_borrowing_users(
                &mut market,
                &mut staking_pool_state,
                1,
                borrow_split.clone(),
                deposit_collateral,
                now_timestamp,
            );

            staking_pool_operations::user_harvest(&mut staking_pool_state, &mut user).unwrap();

            let treasury_fee = borrow_split.fees_to_pay * 1_500 / 10_000;
            let staking_fee = borrow_split.fees_to_pay - treasury_fee;

            let extra_reward = ((staking_fee as u128) * DECIMAL_PRECISION + prev_loss)
                / staking_pool_state.total_stake;

            assert_eq!(user.user_stake, hbb_deposited as u128);
            assert_eq!(
                staking_pool_state.reward_per_token,
                intermediary_reward_token + extra_reward,
            );

            intermediary_reward_token += extra_reward;

            assert_eq!(
                user.rewards_tally,
                intermediary_reward_token * (hbb_deposited as u128),
            );

            prev_loss = staking_pool_state.prev_reward_loss;
        }

        let treasury_fee = borrow_split.fees_to_pay * 1_500 / 10_000;
        let staking_fee = borrow_split.fees_to_pay - treasury_fee;

        assert_eq!(
            staking_pool_state.total_stake,
            (hbb_deposited * count) as u128
        );
        assert_eq!(
            staking_pool_state.total_distributed_rewards,
            (staking_fee * count) as u128
        );
    }

    #[test]
    fn test_staking_harvest_through_unstake_multiple() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut market = BorrowingMarketState::default();

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let count = 100;
        let amount_to_borrow = USDH::from(200.0);
        let hbb_deposited = HBB::from(100.0);
        let deposit_collateral = SOL::from(15.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let now_timestamp = 0;

        for _ in 0..count {
            let mut user = UserStakingState::default();
            staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();
            staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited);

            utils::new_borrowing_users(
                &mut market,
                &mut staking_pool_state,
                1,
                borrow_split.clone(),
                deposit_collateral,
                now_timestamp,
            );

            let UnstakeEffects {
                amount_to_withdraw: _,
                reward,
            } = staking_pool_operations::user_unstake(
                &mut staking_pool_state,
                &mut user,
                hbb_deposited,
            )
            .unwrap();
            let treasury_fee = borrow_split.fees_to_pay * 1_500 / 10_000;
            let staking_fee = borrow_split.fees_to_pay - treasury_fee;
            assert_eq!(reward, staking_fee as u128);
            assert_eq!(user.user_stake, 0);
            assert_eq!(user.rewards_tally, 0);
        }

        let treasury_fee = borrow_split.fees_to_pay * 1_500 / 10_000;
        let staking_fee = borrow_split.fees_to_pay - treasury_fee;
        assert_eq!(staking_pool_state.total_stake, 0);
        assert_eq!(
            staking_pool_state.total_distributed_rewards,
            (staking_fee * count) as u128
        );
    }

    #[test]
    fn test_staking_double_stake_same_user() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut user = UserStakingState::default();
        let mut market = BorrowingMarketState::default();
        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let amount_to_borrow = USDH::from(200.0);
        let hbb_deposited = HBB::from(100.0);
        let deposit_collateral = SOL::from(15.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let now_timestamp = 0;

        staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user).unwrap();
        staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited);

        assert_eq!(user.rewards_tally, 0);

        utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            1,
            borrow_split.clone(),
            deposit_collateral,
            now_timestamp,
        );

        let treasury_fee = borrow_split.fees_to_pay * 1_500 / 10_000;
        let staking_fee = borrow_split.fees_to_pay - treasury_fee;

        let reward_scaled = (staking_fee as u128) * DECIMAL_PRECISION;
        assert_eq!(
            staking_pool_state.reward_per_token,
            reward_scaled / (hbb_deposited as u128),
        );

        staking_pool_operations::user_stake(&mut staking_pool_state, &mut user, hbb_deposited / 2);

        // Before harvest, rewards tally is only modified by the staking done by the user
        let fees_to_pay = staking_fee as u128;
        assert_eq!(user.rewards_tally, fees_to_pay * DECIMAL_PRECISION / 2);

        let HarvestEffects { reward } =
            staking_pool_operations::user_harvest(&mut staking_pool_state, &mut user).unwrap();

        assert_eq!(reward, staking_fee as u128);

        println!("user {:}", user.to_state_string());
        println!("staking pool {:}", staking_pool_state.to_state_string());

        assert_eq!(user.user_stake, (hbb_deposited as u128) * 3 / 2);
        assert_eq!(user.rewards_tally, fees_to_pay * DECIMAL_PRECISION * 3 / 2);

        assert_eq!(
            staking_pool_state.total_stake,
            (hbb_deposited as u128) * 3 / 2
        );
        assert_eq!(staking_pool_state.rewards_not_yet_claimed, 0);
        assert_eq!(staking_pool_state.total_distributed_rewards, fees_to_pay);
        assert_eq!(
            staking_pool_state.reward_per_token,
            fees_to_pay * DECIMAL_PRECISION / hbb_deposited as u128,
        );
    }

    #[test]
    fn test_staking_unstake_more_than_deposited() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let hbb_deposited = HBB::from(100.0);
        let mut users = utils::new_staking_users(&mut staking_pool_state, 2, hbb_deposited);

        // User wants to unstake more than he deposited, but he only receives his fair share
        staking_pool_operations::user_unstake(
            &mut staking_pool_state,
            &mut users[0],
            hbb_deposited * 2,
        )
        .unwrap();

        assert_eq!(users[0].user_stake, 0);
        assert_eq!(users[0].rewards_tally, 0);
        assert_eq!(users[1].user_stake, hbb_deposited as u128);
        assert_eq!(staking_pool_state.total_stake, hbb_deposited as u128);
    }

    #[test]
    fn test_staking_harvest_with_many_loans() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut market = BorrowingMarketState::default();

        let amount_to_borrow = USDH::from(200.0);
        let hbb_deposited = HBB::from(100.0);
        let deposit_collateral = SOL::from(15.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);
        let now_timestamp = 0;

        let count_staked = 3;
        let count_borrowed = 9;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);

        let mut users =
            utils::new_staking_users(&mut staking_pool_state, count_staked, hbb_deposited);

        utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            count_borrowed,
            borrow_split.clone(),
            deposit_collateral,
            now_timestamp,
        );

        assert_eq!(users[0].rewards_tally, 0);

        let HarvestEffects {
            reward: reward_u_one,
        } = staking_pool_operations::user_harvest(&mut staking_pool_state, &mut users[0]).unwrap();

        let treasury_fee = borrow_split.fees_to_pay * 1_500 / 10_000;
        let staking_fee = borrow_split.fees_to_pay - treasury_fee;
        assert_eq!(reward_u_one, 3 * staking_fee as u128);

        assert_eq!(
            users[0].rewards_tally / users[0].user_stake,
            (9 * staking_fee as u128 * DECIMAL_PRECISION) / (3 * hbb_deposited as u128)
        );

        let HarvestEffects {
            reward: reward_u_two,
        } = staking_pool_operations::user_harvest(&mut staking_pool_state, &mut users[1]).unwrap();
        assert_eq!(reward_u_two, 3 * staking_fee as u128);

        println!("staking pool {:}", staking_pool_state.to_state_string());

        let HarvestEffects {
            reward: reward_u_three,
        } = staking_pool_operations::user_harvest(&mut staking_pool_state, &mut users[2]).unwrap();

        // minor precision loss
        assert_eq!(reward_u_three, 3 * staking_fee as u128);

        println!("user one {:}", users[0].to_state_string());
        println!("user two {:}", users[1].to_state_string());
        println!("staking pool {:}", staking_pool_state.to_state_string());

        assert_eq!(users[0].user_stake, hbb_deposited as u128);
        assert_eq!(users[1].user_stake, hbb_deposited as u128);
        assert_eq!(users[2].user_stake, hbb_deposited as u128);

        // minor precision loss, could be saved with rewards_loss
        assert_eq!(staking_pool_state.rewards_not_yet_claimed, 0);
        assert_eq!(
            staking_pool_state.total_distributed_rewards,
            (9 * staking_fee as u128)
        );
        assert_eq!(staking_pool_state.total_stake, hbb_deposited as u128 * 3);
    }

    #[test]
    fn test_staking_double_stake_two_users() {
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let mut user_one = UserStakingState::default();
        let mut user_two = UserStakingState::default();
        let mut market = BorrowingMarketState::default();
        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);
        staking_pool_operations::initialize_staking_pool(&mut staking_pool_state);
        staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user_one).unwrap();
        staking_pool_operations::approve_new_user(&mut staking_pool_state, &mut user_two).unwrap();

        let hbb_deposited = HBB::from(100.0);
        let deposit_collateral = SOL::from(15.0);
        let amount_to_borrow = USDH::from(200.0);
        let borrow_split = BorrowSplit::from_amount(amount_to_borrow, market.base_rate_bps);

        staking_pool_operations::user_stake(&mut staking_pool_state, &mut user_one, hbb_deposited);
        utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            1,
            borrow_split.clone(),
            deposit_collateral,
            now_timestamp,
        );
        staking_pool_operations::user_stake(&mut staking_pool_state, &mut user_two, hbb_deposited);

        let treasury_fee = borrow_split.fees_to_pay * 1_500 / 10_000;
        let staking_fee = borrow_split.fees_to_pay - treasury_fee;

        // User two is not entitled to the Reward distributed == total amount distributed beforehand
        assert_eq!(
            user_two.rewards_tally,
            staking_fee as u128 * DECIMAL_PRECISION
        );

        utils::new_borrowing_users(
            &mut market,
            &mut staking_pool_state,
            1,
            borrow_split.clone(),
            deposit_collateral,
            now_timestamp,
        );

        let res_one = staking_pool_operations::user_harvest(&mut staking_pool_state, &mut user_one);
        match res_one {
            Ok(res) => {
                assert_eq!(res.reward, staking_fee as u128 * 3 / 2);
            }
            Err(e) => println!("Error {}", e),
        }

        let res_two = staking_pool_operations::user_harvest(&mut staking_pool_state, &mut user_two);
        match res_two {
            Ok(res) => {
                assert_eq!(res.reward, staking_fee as u128 / 2);
            }
            Err(e) => println!("Error {}", e),
        }

        assert_eq!(
            user_one.rewards_tally,
            staking_fee as u128 * DECIMAL_PRECISION * 3 / 2
        ); // 50% of 2nd reward + 100% of the first reward

        assert_eq!(staking_pool_state.total_stake, 2 * hbb_deposited as u128);
        assert_eq!(staking_pool_state.rewards_not_yet_claimed, 0);
        assert_eq!(
            staking_pool_state.total_distributed_rewards,
            2 * staking_fee as u128
        );

        assert_eq!(
            staking_pool_state.reward_per_token,
            staking_fee as u128 * DECIMAL_PRECISION * 3 / 2 / hbb_deposited as u128,
        ); // reward/hbb_deposited + reward / 2 * hbb_deposited
    }
}

#[cfg(test)]
mod utils {

    use crate::borrowing_market::borrowing_rate::BorrowSplit;
    use crate::utils::coretypes::{HBB, USDH};
    use crate::UserMetadata;
    use crate::{
        borrowing_market::borrowing_operations, BorrowingMarketState, CollateralToken, TokenPrices,
    };

    use crate::{
        staking_pool::staking_pool_operations, utils::math, StakingPoolState, UserStakingState,
    };
    #[allow(dead_code)]
    const SE: u64 = 10;

    pub fn new_staking_users(
        staking_pool_state: &mut StakingPoolState,
        count: usize,
        staked_hbb: u64,
    ) -> Vec<UserStakingState> {
        (0..count)
            .map(|_| {
                let mut user = UserStakingState::default();
                staking_pool_operations::approve_new_user(staking_pool_state, &mut user).unwrap();
                staking_pool_operations::user_stake(staking_pool_state, &mut user, staked_hbb);
                user
            })
            .collect()
    }

    pub fn new_borrowing_users(
        market: &mut BorrowingMarketState,
        staking_pool_state: &mut StakingPoolState,
        count: usize,
        borrow_split: BorrowSplit,
        deposit_collateral: u64,
        now_timestamp: u64,
    ) -> Vec<UserMetadata> {
        (0..count)
            .map(|_| {
                let mut user = UserMetadata::default();
                borrowing_operations::approve_trove(market, &mut user).unwrap();

                borrowing_operations::deposit_collateral(
                    market,
                    &mut user,
                    deposit_collateral,
                    CollateralToken::SOL,
                )
                .unwrap();

                borrowing_operations::borrow_stablecoin(
                    market,
                    &mut user,
                    staking_pool_state,
                    borrow_split.amount_to_borrow - borrow_split.fees_to_pay,
                    &TokenPrices::new(40.0),
                    now_timestamp,
                )
                .unwrap();

                user
            })
            .collect()
    }

    pub fn _assert_balances(
        users: Vec<&UserStakingState>,
        staking_pool_state: &StakingPoolState,
        expected_total_hbb_staked: f64,
        expected_total_rewards_distributed: f64,
        expected_user_hbb_staked: Vec<f64>,
    ) {
        let actual_total_user_deposits = &staking_pool_state.total_stake;
        assert_eq!(
            *actual_total_user_deposits,
            HBB::from(expected_total_hbb_staked) as u128
        );

        assert_eq!(
            staking_pool_state.total_distributed_rewards,
            USDH::from(expected_total_rewards_distributed) as u128
        );

        for (i, user) in users.iter().enumerate() {
            let actual_user_stake = &user.user_stake;
            assert_eq!(
                *actual_user_stake,
                HBB::from(expected_user_hbb_staked[i]) as u128
            );
        }

        for (i, user) in users.iter().enumerate() {
            let actual_user_reward =
                user.user_stake as u128 * staking_pool_state.reward_per_token - user.rewards_tally;
            assert_eq!(
                actual_user_reward,
                math::hbb_decimal_to_u64(expected_user_hbb_staked[i]) as u128
            );
        }
    }
}
