#![allow(unaligned_references)]
#[cfg(test)]
mod tests {
    use crate::borrowing_market::borrowing_operations;
    use crate::borrowing_market::borrowing_operations::redistribution::compute_new_stake;
    use crate::borrowing_market::borrowing_rate::BorrowSplit;
    use crate::borrowing_market::tests_utils::utils::{
        new_borrower, new_borrowing_users_with_amounts_and_price,
    };
    use crate::redemption::redemption_operations;
    use crate::redemption::test_redemptions::utils::{
        self, add_fill_and_clear_order, fill_redemption_order_new_fillers, new_approved_user,
        new_borrowing_users_with_sol_collateral, new_redemption_orders,
        set_up_filled_redemption_order, setup_redemption_borrowing_program,
        setup_redemption_borrowing_program_with_prices, BorrowersFilter, FilledOrderSetUp,
    };
    use crate::state::epoch_to_scale_to_sum::EpochToScaleToSum;
    use std::cell::RefCell;
    use std::convert::TryInto;

    use crate::{
        assert_fuzzy_eq, BorrowingMarketState, LiquidationsQueue, RedemptionsQueue,
        StabilityPoolState, StakingPoolState,
    };

    use crate::redemption::types::ClearRedemptionOrderEffects;
    use crate::state::redemptions_queue::RedemptionOrderStatus;
    use crate::state::CollateralToken;
    use crate::utils::consts::{BOOTSTRAP_PERIOD, REDEMPTIONS_SECONDS_TO_FILL_ORDER};
    use crate::utils::coretypes::{SOL, USDH};
    use crate::utils::finance::CollateralInfo;
    use crate::{state::CandidateRedemptionUser, UserMetadata};
    use crate::{BorrowError, CollateralAmounts, TokenPrices};
    use decimal_wad::ratio::Ratio;
    use rand::prelude::SliceRandom;
    use rand::thread_rng;
    use solana_sdk::native_token::LAMPORTS_PER_SOL;
    use solana_sdk::pubkey::Pubkey;

    /*

    - create users with collateral ratios:
        - everyone has debt of 100
        - create with coll [100, 90, 80, 70, ... 10]
        - fill with lowest [0, 1, 2] then fill with [0, 0, 0] -> [0, 1, 2]
        - fill with lowest [0, 1, 2] then fill with [1, 1, 1] -> [0, 1, 2]

    */
    #[derive(Debug, Clone)]
    pub struct RedemptionOrderInfo {
        pub redeemer: UserMetadata,
        pub order_id: u64,
    }

    #[test]
    fn test_fill_redemption_empty() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 31;
        let now_timestamp = 0;

        let _ = new_borrowing_users_with_sol_collateral(
            count,
            (0..count)
                .rev()
                .map(|i| ((i + 1) as f64) * 1000.0)
                .collect(),
            &mut market,
            &mut staking_pool_state,
            3000.0,
            now_timestamp,
        );

        let redeem_amt = USDH::from(2500.0);
        let [order_1, _order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let [_bot_1]: [UserMetadata; 1] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            vec![vec![]],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<CandidateRedemptionUser> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| candidate.clone())
            .collect();

        assert_eq!(active_candidates.len(), 0);
    }

    #[ignore]
    #[test]
    fn test_fill_clear_wrong_redemption_order() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 31;
        let now_timestamp = 0;
        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 100.0).collect(),
            &mut market,
            &mut staking_pool_state,
            1000.0,
            now_timestamp,
        );

        let redeem_amt = USDH::from(6000.0);
        let [_order_1, mut order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        // try to fill second redemption order first
        let res = utils::fill_redemption_order(
            &order_2,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        );
        assert_eq!(res.err().unwrap(), BorrowError::InvalidRedemptionOrder);

        // try to clear second redemption order first
        let mut clearer = utils::new_approved_user(&mut market);
        let res = redemption_operations::clear_redemption_order(
            order_2.order_id,
            &mut order_2.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut vec![],
            now_timestamp + (REDEMPTIONS_SECONDS_TO_FILL_ORDER + 5),
        );
        assert_eq!(res.err().unwrap(), BorrowError::InvalidRedemptionOrder);
    }

    #[test]
    fn test_clear_wrong_redeemer() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 31;
        let now_timestamp = 0;
        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 100.0).collect(),
            &mut market,
            &mut staking_pool_state,
            1000.0,
            now_timestamp,
        );

        let redeem_amt = USDH::from(6000.0);
        let [order_1, mut order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        // fill first order
        utils::fill_redemption_order(
            &order_1,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        )
        .unwrap();

        // clear first order - wrong redeemer
        let mut clearer = utils::new_approved_user(&mut market);
        let res = redemption_operations::clear_redemption_order(
            order_1.order_id,
            &mut order_2.redeemer, // wrong redeemer
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut vec![],
            now_timestamp + (REDEMPTIONS_SECONDS_TO_FILL_ORDER + 5),
        );
        assert_eq!(res.err().unwrap(), BorrowError::InvalidRedeemer);
    }

    #[test]
    fn test_fill_redemption_zeros() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 100;
        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            200.0,
            now_timestamp,
        );

        let [order_1, _order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let mut clone_one = borrowers.clone();
        let mut clone_two = borrowers.clone();
        let mut borrowers_one: Vec<&mut UserMetadata> = clone_one.iter_mut().map(|x| x).collect();
        let mut borrowers_two: Vec<&mut UserMetadata> = clone_two.iter_mut().map(|x| x).collect();

        let zero_one = vec![borrowers_one.remove(0)];
        let zero_two = vec![borrowers_two.remove(0)];
        let zero_three = vec![&mut borrowers[0]];

        let [bot_1, _bot_2, _bot_3]: [UserMetadata; 3] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            vec![zero_one, zero_two, zero_three], // vec![vec![0; 64]],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<CandidateRedemptionUser> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| candidate.clone())
            .collect();

        let bot_1_candidates: Vec<u64> = active_candidates
            .iter()
            .filter(|candidate| candidate.filler_metadata == bot_1.metadata_pk)
            .map(|candidate| candidate.user_id)
            .collect();

        assert_eq!(active_candidates.len(), 1);
        let expected: Vec<u64> = vec![0];
        assert_eq!(expected, bot_1_candidates)
    }

    #[test]
    fn test_fill_redemption_duplicate_candidates_once() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 100;
        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        let (borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            200.0,
            now_timestamp,
        );

        let [order_1, _order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let mut clone_one = borrowers.clone();
        let mut clone_two = borrowers.clone();
        let mut borrowers_one: Vec<&mut UserMetadata> = clone_one.iter_mut().map(|x| x).collect();
        let mut borrowers_two: Vec<&mut UserMetadata> = clone_two.iter_mut().map(|x| x).collect();

        let nine_ten_once = vec![borrowers_one.remove(9), borrowers_one.remove(9)];

        let [bot_1]: [UserMetadata; 1] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            // vec![vec![9, 10, 9, 10, 9, 10, 9, 10, 9, 10]],
            vec![nine_ten_once],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let nine_ten_twice = vec![borrowers_two.remove(9), borrowers_two.remove(9)];

        let [_bot_2]: [UserMetadata; 1] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            // vec![vec![9, 10, 9, 10, 9, 10, 9, 10, 9, 10]],
            vec![nine_ten_twice],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let _result_users: Vec<u64> = redemptions_queue.borrow().orders[0]
            .candidate_users
            .iter()
            .filter(|candidate| {
                candidate.status != 0 && candidate.filler_metadata == bot_1.metadata_pk
            })
            .map(|candidate| candidate.user_id)
            .collect();

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<CandidateRedemptionUser> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| candidate.clone())
            .collect();

        let bot_1_candidates: Vec<u64> = active_candidates
            .iter()
            .filter(|candidate| candidate.filler_metadata == bot_1.metadata_pk)
            .map(|candidate| candidate.user_id)
            .collect();

        assert_eq!(active_candidates.len(), 2);
        let expected: Vec<u64> = vec![10, 9];
        assert_eq!(expected, bot_1_candidates)
    }

    #[test]
    fn test_fill_redemption_duplicate_candidates_filled_multiple_times() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 100;
        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        let (borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            200.0,
            now_timestamp,
        );

        let [order_1, _order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let mut clone_one = borrowers.clone();
        let mut borrowers_one: Vec<&mut UserMetadata> = clone_one.iter_mut().map(|x| x).collect();
        let nine_ten_once = vec![borrowers_one.remove(9), borrowers_one.remove(9)];

        let [bot_1]: [UserMetadata; 1] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            vec![nine_ten_once],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        for _ in 0..5 {
            let mut clone_two = borrowers.clone();
            let mut borrowers_two: Vec<&mut UserMetadata> =
                clone_two.iter_mut().map(|x| x).collect();

            let nine_ten_many = vec![borrowers_two.remove(9), borrowers_two.remove(9)];

            let [_bot_2]: [UserMetadata; 1] = fill_redemption_order_new_fillers(
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                order_1.order_id,
                vec![nine_ten_many],
                now_timestamp,
            )
            .try_into()
            .unwrap();
        }

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<CandidateRedemptionUser> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| candidate.clone())
            .collect();

        let bot_1_candidates: Vec<u64> = active_candidates
            .iter()
            .filter(|candidate| candidate.filler_metadata == bot_1.metadata_pk)
            .map(|candidate| candidate.user_id)
            .collect();

        assert_eq!(active_candidates.len(), 2);
        let expected: Vec<u64> = vec![10, 9];
        assert_eq!(expected, bot_1_candidates)
    }

    #[test]
    fn test_fill_redemption_same_candidates_different_order() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 100;
        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        let (borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            200.0,
            now_timestamp,
        );

        let [order_1, _order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let mut clone_one = borrowers.clone();
        let mut clone_two = borrowers.clone();
        let mut borrowers_one: Vec<&mut UserMetadata> = clone_one.iter_mut().map(|x| x).collect();
        let mut borrowers_two: Vec<&mut UserMetadata> = clone_two.iter_mut().map(|x| x).collect();

        let users_order_one = vec![
            borrowers_one.remove(10), // user_id 10
            borrowers_one.remove(19), // user_id 20
            borrowers_one.remove(28), // user_id 30
        ];

        let users_order_two = vec![
            borrowers_two.remove(10), // user_id 10
            borrowers_two.remove(29), // user_id 30
            borrowers_two.remove(19), // user_id 20
        ];

        let [bot_1, _bot_2]: [UserMetadata; 2] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            vec![users_order_one, users_order_two],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<CandidateRedemptionUser> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| candidate.clone())
            .collect();

        let bot_1_candidates: Vec<u64> = active_candidates
            .iter()
            .filter(|candidate| candidate.filler_metadata == bot_1.metadata_pk)
            .map(|candidate| candidate.user_id)
            .collect();

        assert_eq!(active_candidates.len(), 3);
        assert_eq!(vec![30, 20, 10], bot_1_candidates);
    }

    #[test]
    fn test_fill_redemption_same_candidates_second_contains_dupes() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        let count = 100;
        let (borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            200.0,
            now_timestamp,
        );

        let mut borrowers_clone_one = borrowers.clone();
        let mut borrowers_clone_two = borrowers.clone();
        let mut borrowers_clone_three = borrowers.clone();
        let [order_1, _order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let mut borrowers_mut_one: Vec<&mut UserMetadata> =
            borrowers_clone_one.iter_mut().map(|x| x).collect();
        let users_one = vec![
            borrowers_mut_one.remove(10), // user_id 10
            borrowers_mut_one.remove(19), // user_id 20
            borrowers_mut_one.remove(28), // user_id 30
        ];

        let [bot_1]: [UserMetadata; 1] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            vec![users_one],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        // clone to have duplicates
        let mut borrowers_mut_two: Vec<&mut UserMetadata> =
            borrowers_clone_two.iter_mut().map(|x| x).collect();
        let mut borrowers_mut_three: Vec<&mut UserMetadata> =
            borrowers_clone_three.iter_mut().map(|x| x).collect();

        let mut users_two = vec![
            borrowers_mut_two.remove(10),
            borrowers_mut_two.remove(19),
            borrowers_mut_three.remove(10),
            borrowers_mut_three.remove(19),
        ];

        // duplicates is error
        let fill_bot = new_approved_user(&mut market);
        let res = redemption_operations::fill_redemption_order(
            order_1.order_id,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut users_two,
            &fill_bot,
            now_timestamp,
        );
        assert_eq!(res.err().unwrap(), BorrowError::DuplicateAccountInFillOrder);

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<CandidateRedemptionUser> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| candidate.clone())
            .collect();

        let bot_1_candidates: Vec<u64> = active_candidates
            .iter()
            .filter(|candidate| candidate.filler_metadata == bot_1.metadata_pk)
            .map(|candidate| candidate.user_id)
            .collect();

        assert_eq!(active_candidates.len(), 3);
        assert_eq!(vec![30, 20, 10], bot_1_candidates);
    }

    fn chunks(borrowers: &mut Vec<UserMetadata>, num: usize) -> Vec<Vec<&mut UserMetadata>> {
        let mut borrowers_mut: Vec<&mut UserMetadata> = borrowers.iter_mut().map(|x| x).collect();
        let mut all_chunks: Vec<Vec<&mut UserMetadata>> = vec![];
        borrowers_mut.drain(..).enumerate().for_each(|(i, x)| {
            let chunk_idx = i / num;
            if all_chunks.len() > chunk_idx {
                all_chunks[chunk_idx].push(x);
            } else {
                all_chunks.push(vec![x]);
            }
        });

        all_chunks
    }

    fn flatten<T>(nested: Vec<Vec<T>>) -> Vec<T> {
        nested.into_iter().flatten().collect()
    }

    /*
        Chunks => [[0, 1, 2], [3, 4, 5], [6, 7, 8].. [96, 97, 98]]
        Expected => [98, 97, 96, 95.. 66]
    */
    #[test]
    fn test_fill_redemption_better_candidates_subsequent_fillers() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 99;
        let redeem_amt = USDH::from(6000.0);
        let now_timestamp = 0;
        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 100.0).collect(),
            &mut market,
            &mut staking_pool_state,
            1000.0,
            now_timestamp,
        );

        let [order_1]: [RedemptionOrderInfo; 1] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let chunks = chunks(&mut borrowers, 3);
        let fillers: [UserMetadata; 33] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            chunks,
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<(u64, Pubkey)> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| (candidate.user_id, candidate.filler_metadata))
            .collect();

        let (mut j, mut k) = (1, 1);
        let mut expected = vec![];
        while j < all_candidates.len() {
            let mut l = j;
            while l < j + 3 {
                if l <= all_candidates.len() {
                    expected.push(((count - l) as u64, fillers[fillers.len() - k].metadata_pk));
                }
                l += 1;
            }
            j += 3;
            k += 1;
        }
        assert_eq!(active_candidates.len(), 32);
        assert_eq!(active_candidates, expected);
    }

    /*
        Chunks => [[98, 97, 96], [95, 94, 93], [92, 91, 90].. [2, 1, 0]]
        Expected => [98, 97, 96, 95.. 66]
    */
    #[test]
    fn test_fill_redemption_worse_candidates_subsequent_fillers() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 99;
        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        let (borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            200.0,
            now_timestamp,
        );

        let [order_1]: [RedemptionOrderInfo; 1] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let mut borrowers: Vec<UserMetadata> = borrowers.into_iter().rev().collect();
        let chunks = chunks(&mut borrowers, 3);

        let fillers: [UserMetadata; 33] = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            chunks,
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<(u64, Pubkey)> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| (candidate.user_id, candidate.filler_metadata))
            .collect();

        let (mut j, mut k) = (1, 0);
        let mut expected = vec![];
        while j < all_candidates.len() {
            let mut l = j;
            while l < j + 3 {
                if l <= all_candidates.len() {
                    expected.push(((count - l) as u64, fillers[k].metadata_pk));
                }
                l += 1;
            }
            j += 3;
            k += 1;
        }
        assert_eq!(active_candidates.len(), 32);
        assert_eq!(active_candidates, expected);
    }

    /*
        Chunks => [[0, 1, 2], [2, 3, 4], [4, 5, 6].. [96, 97, 98]]
        Expected => [98, 97, 96, 95.. 66]
    */
    #[test]
    fn test_fill_redemption_better_candidates_subsequent_fillers_overlapping() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 99;
        let redeem_amt = USDH::from(6000.0);
        let now_timestamp = 0;
        let (borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 100.0).collect(),
            &mut market,
            &mut staking_pool_state,
            1000.0,
            now_timestamp,
        );

        let [order_1]: [RedemptionOrderInfo; 1] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let user_ids: Vec<u8> = (0..count as u8).collect();
        let mut ch: Vec<Vec<UserMetadata>> = vec![];
        let mut n = 0;
        while n < user_ids.len() - 2 {
            ch.push(vec![
                borrowers[n].clone(),
                borrowers[n + 1].clone(),
                borrowers[n + 2].clone(),
            ]);
            n += 2;
        }

        let mut ch = flatten(ch);
        let chunks = chunks(&mut ch, 3);

        let fillers: Vec<UserMetadata> = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            chunks,
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<(u64, Pubkey)> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| (candidate.user_id, candidate.filler_metadata))
            .collect();

        let (mut j, mut k) = (1, 1);
        let mut expected = vec![];
        while j < all_candidates.len() {
            let mut l = j;
            while l < j + 2 {
                if l <= all_candidates.len() {
                    expected.push(((count - l) as u64, fillers[fillers.len() - k].metadata_pk));
                }
                l += 1;
            }
            j += 2;
            k += 1;
        }
        assert_eq!(active_candidates.len(), 32);
        assert_eq!(active_candidates, expected);
    }

    /*
        Chunks => [[98, 97, 96], [96, 95, 94], [93, 92, 91].. [2, 1, 0]]
        Expected => [98, 97, 96, 95.. 66]
    */
    #[test]
    fn test_fill_redemption_worse_candidates_subsequent_fillers_overlapping() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 99;
        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        let (borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            200.0,
            now_timestamp,
        );

        let [order_1]: [RedemptionOrderInfo; 1] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let user_ids: Vec<u8> = (0..count as u8).collect();
        let mut ch: Vec<Vec<UserMetadata>> = vec![];
        let mut n = 0;
        while n < user_ids.len() - 2 {
            ch.push(vec![
                borrowers[n].clone(),
                borrowers[n + 1].clone(),
                borrowers[n + 2].clone(),
            ]);
            n += 2;
        }

        let mut ch = flatten(ch);
        let chunks = chunks(&mut ch, 3);

        let fillers: Vec<UserMetadata> = fill_redemption_order_new_fillers(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            order_1.order_id,
            chunks,
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let all_candidates = redemptions_queue.borrow().orders[0].candidate_users;
        let active_candidates: Vec<(u64, Pubkey)> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| (candidate.user_id, candidate.filler_metadata))
            .collect();

        let (mut j, mut k) = (1, 1);
        let mut expected = vec![];
        while j < all_candidates.len() {
            let mut l = j;
            while l < j + 2 {
                if l <= all_candidates.len() {
                    expected.push(((count - l) as u64, fillers[fillers.len() - k].metadata_pk));
                }
                l += 1;
            }
            j += 2;
            k += 1;
        }
        assert_eq!(active_candidates.len(), 32);
        assert_eq!(active_candidates, expected);
    }

    /*
    - [x] test 1 user fill, 1 user clear, one at a time until all is done
    - [x] trying to clear with empty is error
    - [x] test clear with wrong users is failure
    - [x] trying to clear with mismatched candidates is error
    - [x] test fully clear makes the order open again

    - [x] test clear while filling before time elapses is wrong
    - [x] skipping users in the right order is forbidden (cannot redeem user 2 without first redeeming user 1)
    - [x] assert cannot fill until clearing is done
    - [x] test filling while clearing is wrong
    - [x] multiple rounds of filling & redeeming / semaphore
    - [x] test fully redeemed users and
    - [x] test partially redeemed users

    - [x] test adding 5 orders and trying to fill/clear 2, 3, 4, 5 is broken
    - [x] multiple orders, ensure all are cleared in good order, all partially filled
    - [x] test filling / clearing with wrong filling bot is error
    - [x] filling again with already redeemed users should fail (noop since infinite CR)
        - [x] should not be allowed to redeem users with no debt
    - [x] test 10 users submitted, 5 users cleared, 5 users cleared again, not more not less

    - [x] attribute to correct bots test
        - [x] multiple bots/same bot

    - recalculate position test
        - test between withdraw/deposit and fill/claim
        - test that user is unchanged if there is a "top up" after bot filling order with him
        - test user has updated position in between 2 fill orders -> second one is allowed, first is ignored

    - TODO: apply pendinng rewards
    - test redistributions
    - test after liquidation

    // TODO: convert RedemptionOrder to UserMetadata and only keep track of index
    // so we don't have a massive array, we can instead just have an array of indices and statuses ? can be hacked?
    // same with liquidations queue

    */
    #[test]
    fn test_redemption_clear_order_simple() {
        // 1. Add one order
        // 2. Fill it correctly
        // 3. Clear it
        // 4. Ensure queue is empty

        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // redeeming 600.0
        // everyone has 100.4999
        // that means we will redeem 600.0 / 100.4999 = 5.970155194184273 - 6 users
        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        // 0 has the lowest CR, 99 has the highest CR
        let count = 99;
        let borrow_per_user = 200.0;
        let collateral_sol_deposits = (0..count).map(|i| ((i + 1) as f64) * 10.0).collect();
        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            collateral_sol_deposits,
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );
        let collateral_infos_before: Vec<CollateralInfo> = borrowers
            .iter()
            .map(|borrower| CollateralInfo::from(borrower, &prices))
            .collect();

        let total_collateral_before = borrowers
            .iter()
            .fold(CollateralAmounts::default(), |acc, borrower| {
                acc.add(&borrower.deposited_collateral)
            });

        let [mut order_1]: [RedemptionOrderInfo; 1] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let mut submitted_users = borrowers.clone();
        submitted_users.shuffle(&mut thread_rng());

        let mut fill_bot =
            crate::redemption::test_redemptions::utils::new_approved_user(&mut market);

        let mut submitted_users_chunks = chunks(&mut submitted_users, 5);

        submitted_users_chunks
            .iter_mut()
            .for_each(|submitted_users_chunk| {
                redemption_operations::fill_redemption_order(
                    order_1.order_id,
                    &mut market,
                    &mut redemptions_queue.borrow_mut(),
                    submitted_users_chunk,
                    &fill_bot,
                    now_timestamp,
                )
                .unwrap();
            });

        let mut clearer =
            crate::redemption::test_redemptions::utils::new_approved_user(&mut market);

        let mut fillers_and_borrowers: Vec<&mut UserMetadata> = vec![&mut fill_bot];
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        // Try to fill it once, but 5 seconds have not passed by
        let err = redemption_operations::clear_redemption_order(
            order_1.order_id,
            &mut order_1.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp,
        )
        .err();

        assert_eq!(
            err.unwrap(),
            BorrowError::CannotClearRedemptionOrderWhileInFillingMode
        );

        // Try again after REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1 seconds
        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            redeemed_collateral,
        } = redemption_operations::clear_redemption_order(
            order_1.order_id,
            &mut order_1.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();

        println!(
            "Cleared redemption order {}",
            &redemptions_queue.clone().borrow().orders[0].to_state_string()
        );

        assert_eq!(
            redemptions_queue.clone().borrow().orders[0].status,
            RedemptionOrderStatus::Inactive as u8
        );

        // TODO: apply events on the ui too (for redistribution)
        // assert borrowers' balance
        // assert NET VALUE is unchanged for all users

        println!("Redeemer {:?}", order_1.redeemer);
        println!("Clearing Bot {:?}", clearer);
        println!("Filling Bot {:?}", fill_bot);

        assert!(!fill_bot.inactive_collateral.is_zero());
        assert!(!clearer.inactive_collateral.is_zero());

        let collateral_infos_after: Vec<CollateralInfo> = borrowers
            .iter()
            .map(|borrower| {
                CollateralInfo::calculate_collateral_value(
                    borrower.borrowed_stablecoin,
                    &borrower
                        .deposited_collateral
                        .add(&borrower.inactive_collateral),
                    &prices,
                )
            })
            .collect();

        let total_collateral_after =
            borrowers
                .iter()
                .fold(CollateralAmounts::default(), |acc, borrower| {
                    acc.add(&borrower.deposited_collateral)
                        .add(&borrower.inactive_collateral)
                });

        let redeemed_collateral_diffed = total_collateral_before.sub(&total_collateral_after);
        let redeemed_collateral_effects = CollateralAmounts::default()
            .add(&redeemed_collateral.stakers)
            .add(&redeemed_collateral.redeemer)
            .add(&redeemed_collateral.filler)
            .add(&redeemed_collateral.clearer);
        let redeemed_collateral_manual = CollateralAmounts::default()
            .add(&clearer.inactive_collateral)
            .add(&fill_bot.inactive_collateral)
            .add(&order_1.redeemer.inactive_collateral)
            .add(&redeemed_collateral.stakers); // unfortunately we don't side effect this one yet

        println!("Redeemed diff'ed {:?}", redeemed_collateral_diffed);
        println!("Redeemed effects {:?}", redeemed_collateral_effects);
        println!("Redeemed manual {:?}", redeemed_collateral_manual);
        assert_eq!(redeemed_collateral_diffed, redeemed_collateral_effects);
        assert_eq!(redeemed_collateral_diffed, redeemed_collateral_manual);

        for (i, ((borrower, ci_bef), ci_aft)) in borrowers
            .iter()
            .zip(collateral_infos_before.iter())
            .zip(collateral_infos_after.iter())
            .enumerate()
        {
            let borrow_amount = USDH::from(borrow_per_user);
            let borrow_split = BorrowSplit::from_amount(borrow_amount, 0);
            // let redeem_ratio = USDH::from(borrow_per_user * 1.005) as f64 / (ci_bef.collateral_value as f64);
            let collateral_lost =
                ci_bef.collateral_value * borrow_split.amount_to_borrow / ci_bef.collateral_value; //  * redeem_ratio;
            let calced_new_collateral = ci_bef.collateral_value - collateral_lost;
            println!(
                    "Borrower after {} - {} - NV bef {} NV aft {} prev coll {} new coll {} calc coll {}",
                    i,
                    borrower.to_state_string(),
                    ci_bef.net_value,
                    ci_aft.net_value,
                    ci_bef.collateral_value,
                    ci_aft.collateral_value,
                    calced_new_collateral
                );

            // for those fully redeemed
            if i < 5 {
                assert_eq!(borrower.borrowed_stablecoin, 0);
                assert_eq!(ci_aft.collateral_value, calced_new_collateral);
            }

            assert_eq!(ci_bef.net_value, ci_aft.net_value);
        }

        // ensure queue is empty
        for order in redemptions_queue.borrow().orders.iter() {
            assert_eq!(order.status, RedemptionOrderStatus::Inactive as u8);
        }

        assert_eq!(redeemed_stablecoin, redeem_amt);
    }

    #[test]
    fn test_redemption_clear_order_abstracted() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 99;
        let borrow_per_user = 200.0;
        let requested_redeemption_amount = USDH::from(2500.0);
        let now_timestamp = 0;

        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        // for test assertions
        let borrowers_snapshot = borrowers.clone();

        let FilledOrderSetUp {
            mut order,
            mut fill_bot,
            ..
        } = set_up_filled_redemption_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::All,
            &prices,
            requested_redeemption_amount,
            now_timestamp,
        )
        .unwrap();

        let mut clearer = utils::new_approved_user(&mut market);

        let mut fillers_and_borrowers = vec![&mut fill_bot];
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            redeemed_collateral,
        } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();

        utils::assert_global_collateral_unchanged(
            &borrowers_snapshot,
            &borrowers,
            &redeemed_collateral,
            &clearer,
            &vec![fill_bot.clone()],
            &order,
        );
        utils::assert_order_cleared(redemptions_queue.clone(), 0);
        utils::assert_simulation_results_match(
            requested_redeemption_amount,
            market.base_rate_bps,
            &prices,
            &fill_bot,
            &clearer,
            &order.redeemer,
            &borrowers_snapshot,
            &borrowers,
        );
        utils::assert_queue_is_empty(redemptions_queue.clone());
        utils::assert_net_value_unchanged(
            borrow_per_user,
            &prices,
            &borrowers_snapshot,
            &borrowers,
        );
        utils::assert_debt_burned(
            requested_redeemption_amount,
            redeemed_stablecoin,
            &borrowers_snapshot,
            &borrowers,
        );
    }

    #[test]
    fn test_redemption_fill_and_clear_one_at_a_time_repeatedly() {
        // - test 1 user fill, 1 user clear, one at a time until all is done

        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 10;
        let borrow_per_user = 1000.0;
        let requested_redeemption_amount = USDH::from(6000.0);
        let now_timestamp = 0;

        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            vec![100.0; count],
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        // for test assertions
        let borrowers_snapshot = borrowers.clone();

        let mut clearer = utils::new_approved_user(&mut market);

        let mut order = utils::set_up_redemption_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            requested_redeemption_amount,
            now_timestamp,
        );

        let mut total_redeemed_stablecoin_effect = 0;
        let mut total_fill_bot_effect = CollateralAmounts::default();
        let mut total_redeemer_effect = CollateralAmounts::default();

        // First 6 users
        for i in 0..6 {
            let mut fill_bot = utils::fill_redemption_order(
                &order,
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                &mut borrowers,
                utils::BorrowersFilter::Some(vec![i]),
                // TODO, make sure this doesn't work, that time continuously grows
                // maybe we should use epoch instead, need to check if
                // possible at all in solana to have an old transaction
                // with an old timestamp, which would reset the time back
                // solution: only update latest timestamp
                // technically, it's not really a problem because we
                // just set the timestamp the first time a fill is pushed
                now_timestamp,
            )
            .unwrap();

            let mut fillers_and_borrowers: Vec<&mut UserMetadata> = vec![&mut fill_bot];
            borrowers
                .iter_mut()
                .for_each(|user| fillers_and_borrowers.push(user));

            utils::print_candidate_users(redemptions_queue.clone(), 0);

            let ClearRedemptionOrderEffects {
                redeemed_stablecoin,
                redeemed_collateral,
            } = redemption_operations::clear_redemption_order(
                order.order_id,
                &mut order.redeemer,
                &mut clearer,
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                &mut fillers_and_borrowers,
                now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
            )
            .unwrap();

            total_redeemed_stablecoin_effect += redeemed_stablecoin;
            total_fill_bot_effect.add_assign(&redeemed_collateral.filler);
            total_redeemer_effect.add_assign(&redeemed_collateral.redeemer);

            utils::print_candidate_users(redemptions_queue.clone(), 0);
            utils::print_order(
                format!("After clearing round {}", i).as_str(),
                redemptions_queue.clone(),
                0,
            );

            if i < 5 {
                let expected_burned_amount =
                    borrowers_snapshot[0].borrowed_stablecoin * ((i as u64) + 1);
                utils::assert_debt_burned(
                    expected_burned_amount,
                    total_redeemed_stablecoin_effect,
                    &borrowers_snapshot,
                    &borrowers,
                );
            }

            if i == 5 {
                let expected_redeemed_amount =
                    requested_redeemption_amount - borrowers_snapshot[0].borrowed_stablecoin * 5;

                assert_eq!(redemptions_queue.borrow().orders[0].remaining_amount, 0);

                // assert last user is partially redeemed
                let coll_info = CollateralInfo::from(&borrowers_snapshot[5], &prices);
                let ratio = Ratio::new(expected_redeemed_amount, coll_info.collateral_value);
                let expected_redeemed_collateral = borrowers_snapshot[5]
                    .deposited_collateral
                    .mul_fraction(ratio.numerator, ratio.denominator);

                utils::assert_debt_burned(
                    requested_redeemption_amount,
                    total_redeemed_stablecoin_effect,
                    &borrowers_snapshot,
                    &borrowers,
                );

                let redeemed_collateral_effects = CollateralAmounts::default()
                    .add(&redeemed_collateral.stakers)
                    .add(&redeemed_collateral.redeemer)
                    .add(&redeemed_collateral.filler)
                    .add(&redeemed_collateral.clearer);

                assert_eq!(expected_redeemed_collateral, redeemed_collateral_effects);

                utils::assert_order_cleared(redemptions_queue.clone(), 0);
                utils::assert_queue_is_empty(redemptions_queue.clone());
            }

            for (i, user) in borrowers.iter().enumerate() {
                println!("Updated B {} - {:?}", i, user.to_state_string());
            }
        }

        let fill_bot = UserMetadata {
            inactive_collateral: total_fill_bot_effect,
            ..Default::default()
        };
        let redeemer = UserMetadata {
            inactive_collateral: total_redeemer_effect,
            ..Default::default()
        };

        utils::assert_simulation_results_match(
            requested_redeemption_amount,
            market.base_rate_bps,
            &prices,
            &fill_bot,
            &clearer,
            &redeemer,
            &borrowers_snapshot,
            &borrowers,
        );
    }

    #[test]
    fn test_redemption_update_user_between_clear_and_fill() {
        // updating a user after a fill should make it exempt from clear
        // but can still be considered with the new CR (not tested in this test)

        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 100;
        let borrow_per_user = 200.0;
        let requested_redeemption_amount = USDH::from(6000.0);
        let now_timestamp = 0;

        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).map(|i| ((i + 1) as f64) * 10.0).collect(),
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        let mut clearer = utils::new_approved_user(&mut market);

        let mut order = utils::set_up_redemption_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            requested_redeemption_amount,
            now_timestamp,
        );

        let mut fill_bot_one = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        )
        .unwrap();

        // assert first user is a candidate
        let borrowed_before_first = borrowers[0].borrowed_stablecoin;
        let borrowed_before_second = borrowers[1].borrowed_stablecoin;
        assert_eq!(
            redemptions_queue.borrow().orders[0].candidate_users[0].user_id,
            borrowers[0].user_id
        );

        // first user deposit some collateral, change the collateral ratio
        borrowing_operations::deposit_collateral(
            &mut market,
            &mut borrowers[0],
            1 * LAMPORTS_PER_SOL,
            CollateralToken::SOL,
        )
        .unwrap();

        let mut fillers_and_borrowers: Vec<&mut UserMetadata> = vec![&mut fill_bot_one];
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        let ClearRedemptionOrderEffects { .. } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + (REDEMPTIONS_SECONDS_TO_FILL_ORDER + 3),
        )
        .unwrap();

        // assert that the first user, but othrs are changed is unchanged
        let borrowed_after_first = borrowers[0].borrowed_stablecoin;
        let borrowed_after_second = borrowers[1].borrowed_stablecoin;
        assert_eq!(borrowed_after_first, borrowed_before_first);
        assert_eq!(borrowed_before_second, borrowed_before_first);
        assert_eq!(borrowed_after_second, 0);
    }

    #[test]
    fn test_fill_and_clear_one_order_then_the_next() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 31;
        let redeem_amt = USDH::from(2000.0);
        let now_timestamp = 0;
        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).map(|i| ((i + 1) as f64) * 1000.0).collect(),
            &mut market,
            &mut staking_pool_state,
            10000.0,
            now_timestamp,
        );

        let [mut order_1, mut order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let mut filler_one = utils::fill_redemption_order(
            &order_1,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        )
        .unwrap();

        let mut clearer = utils::new_approved_user(&mut market);
        utils::print_borrowers("Before Clear", &borrowers);
        utils::print_order("Before clear 0", redemptions_queue.clone(), 0);
        utils::print_order("Before clear 1", redemptions_queue.clone(), 1);

        let mut fillers_and_borrowers = vec![&mut filler_one];
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            ..
        } = redemption_operations::clear_redemption_order(
            order_1.order_id,
            &mut order_1.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();
        utils::print_borrowers("After Clear", &borrowers);
        utils::print_order("After clear 0", redemptions_queue.clone(), 0);
        utils::print_order("After clear 1", redemptions_queue.clone(), 1);
        assert_eq!(redeemed_stablecoin, redeem_amt);
        assert_eq!(borrowers[0].borrowed_stablecoin, 8050000000);
        utils::assert_order_cleared(redemptions_queue.clone(), 0);

        // Now fill order two
        let mut filler_two = utils::fill_redemption_order(
            &order_2,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![3, 0, 1, 2, 4, 5, 6]),
            now_timestamp,
        )
        .unwrap();

        let mut fillers_and_borrowers = vec![&mut filler_two];
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            ..
        } = redemption_operations::clear_redemption_order(
            order_2.order_id,
            &mut order_2.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();

        // first user still has the lowest collateral ratio
        assert_eq!(redeemed_stablecoin, redeem_amt);
        assert_eq!(borrowers[0].borrowed_stablecoin, 6050000000);
        utils::assert_order_cleared(redemptions_queue.clone(), 1);
    }

    #[test]
    fn test_redemption_fill_and_clear_semaphore() {
        // - test clear while filling before time elapses is wrong
        // - assert cannot fill until clearing is done
        // - test filling while clearing is wrong

        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 10;
        let borrow_per_user = 1000.0;
        let requested_redeemption_amount = USDH::from(6000.0);
        let now_timestamp = 0;

        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).map(|i| ((i + 1) as f64) * 100.0).collect(),
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        // for test assertions
        let borrowers_snapshot = borrowers.clone();

        let mut clearer = utils::new_approved_user(&mut market);

        let mut order = utils::set_up_redemption_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            requested_redeemption_amount,
            now_timestamp,
        );

        let mut fill_bot_one = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        )
        .unwrap();

        utils::print_candidate_users(redemptions_queue.clone(), 0);

        let mut fillers_and_borrowers: Vec<&mut UserMetadata> = vec![&mut fill_bot_one];
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        // Clearing while filling in the time window is an error
        let res = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + (REDEMPTIONS_SECONDS_TO_FILL_ORDER - 1),
        );
        assert!(res.is_err());

        // Partially clearing second user, skipping the first is wrong
        // We should always clear from the bottom up
        // In case we don't provide the first few users, the result should be no-op
        let mut borrowers_mut: Vec<&mut UserMetadata> = borrowers.iter_mut().map(|x| x).collect();

        let mut skipping_first_user = vec![&mut fill_bot_one, borrowers_mut[1]];
        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            ..
        } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut skipping_first_user,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();
        assert_eq!(redeemed_stablecoin, 0);

        // Partially clearing first user, sets it to clearing
        let mut first_filler_and_borrower = vec![&mut fill_bot_one, borrowers_mut[0]];
        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            ..
        } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut first_filler_and_borrower,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();
        utils::print_candidate_users(redemptions_queue.clone(), 0);
        utils::assert_order_status(
            redemptions_queue.clone(),
            0,
            RedemptionOrderStatus::Claiming,
        );

        utils::print_order("After clearing", redemptions_queue.clone(), 0);

        let expected_burned_amount = borrowers_snapshot[0].borrowed_stablecoin;
        utils::assert_debt_burned(
            expected_burned_amount,
            redeemed_stablecoin,
            &borrowers_snapshot,
            &borrowers,
        );

        // Try to fill again, should be an error
        let res = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        )
        .err()
        .unwrap();
        assert_eq!(
            res,
            BorrowError::CannotFillRedemptionOrderWhileInClearingMode
        );

        // Clear the remaining two, make it Open for filling again
        let mut borrowers_clone = borrowers.clone();
        let mut borrowers_mut: Vec<&mut UserMetadata> = borrowers.iter_mut().map(|x| x).collect();
        let mut second_third_borrowers_and_filler = vec![&mut fill_bot_one];
        borrowers_mut
            .drain(1..3)
            .for_each(|x| second_third_borrowers_and_filler.push(x));

        let ClearRedemptionOrderEffects { .. } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut second_third_borrowers_and_filler,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 3,
        )
        .unwrap();
        utils::print_candidate_users(redemptions_queue.clone(), 0);
        utils::assert_order_status(redemptions_queue.clone(), 0, RedemptionOrderStatus::Open);

        utils::print_order("After filling all 3 ", redemptions_queue.clone(), 0);

        let expected_remaining =
            requested_redeemption_amount - &borrowers_snapshot[0].borrowed_stablecoin * 3;
        assert_eq!(
            redemptions_queue.borrow().orders[0].remaining_amount,
            expected_remaining
        );

        // Fill the next 5 users
        // Then clear the queue
        // Assert only the partial amount was used
        let mut fill_bot_two = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers_clone,
            utils::BorrowersFilter::Some(vec![2, 3, 4, 5, 6, 7]),
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 3,
        )
        .unwrap();
        utils::print_candidate_users(redemptions_queue.clone(), 0);
        utils::assert_order_status(redemptions_queue.clone(), 0, RedemptionOrderStatus::Filling);
        utils::print_order("After mext 5 ", redemptions_queue.clone(), 0);

        drop(borrowers_mut);
        let mut second_to_ninth_borrower_and_filler = vec![&mut fill_bot_two];
        let mut borrowers_mut: Vec<&mut UserMetadata> = borrowers.iter_mut().map(|x| x).collect();
        borrowers_mut
            .drain(1..9)
            .for_each(|x| second_to_ninth_borrower_and_filler.push(x));

        // Should be an error, need to fill after seconds have passed
        let res = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut second_to_ninth_borrower_and_filler,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        );
        assert!(res.is_err());

        drop(borrowers_mut);
        let mut second_to_ninth_borrower_and_filler = vec![&mut fill_bot_two];
        let mut borrowers_mut: Vec<&mut UserMetadata> = borrowers.iter_mut().map(|x| x).collect();
        borrowers_mut
            .drain(1..9)
            .for_each(|x| second_to_ninth_borrower_and_filler.push(x));

        let red_q = redemptions_queue.clone();
        let ClearRedemptionOrderEffects { .. } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut second_to_ninth_borrower_and_filler,
            now_timestamp
                + REDEMPTIONS_SECONDS_TO_FILL_ORDER
                + 3
                + REDEMPTIONS_SECONDS_TO_FILL_ORDER
                + 1,
        )
        .unwrap();

        for (i, u) in borrowers.iter().enumerate() {
            println!("Borrowers after {} - {}", i, u.to_state_string());
        }

        // assert order is filled, everything is cleared
        drop(red_q);
        utils::print_candidate_users(redemptions_queue.clone(), 0);
        utils::print_order("After last 5 cleared ", redemptions_queue.clone(), 0);
        utils::assert_order_status(
            redemptions_queue.clone(),
            0,
            RedemptionOrderStatus::Inactive,
        );

        for (i, user) in borrowers.iter().enumerate() {
            println!("Updated B {} - {:?}", i, user.to_state_string());
        }

        // merge fillers
        let mut filler = fill_bot_one.clone();
        filler
            .inactive_collateral
            .add_assign(&fill_bot_two.inactive_collateral);

        utils::assert_simulation_results_match(
            requested_redeemption_amount,
            market.base_rate_bps,
            &prices,
            &filler,
            &clearer,
            &order.redeemer,
            &borrowers_snapshot,
            &borrowers,
        )
    }

    #[test]
    fn test_redemption_partial_mismatched_empty_redemption() {
        // - [x] trying to clear with empty is error
        // - [x] test clear with wrong users is failure
        // - [x] trying to clear with mismatched candidates is error
        // - [x] test fully clear makes the order open again

        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 50;
        let borrow_per_user = 200.0;
        let requested_redeemption_amount = USDH::from(2500.0);
        let now_timestamp = 0;

        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            vec![10.0; count],
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        // for test assertions
        let borrowers_snapshot = borrowers.clone();

        let mut clearer = utils::new_approved_user(&mut market);

        let mut order = utils::set_up_redemption_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            requested_redeemption_amount,
            now_timestamp,
        );

        let mut fill_bot = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0]),
            now_timestamp,
        )
        .unwrap();

        utils::print_candidate_users(redemptions_queue.clone(), 0);

        // Clear incorrectly is error
        let (mut wrong_borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            vec![10.0; count],
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        let mut wrong_filler_and_borrowers: Vec<&mut UserMetadata> = vec![&mut fill_bot];
        wrong_borrowers
            .iter_mut()
            .for_each(|user| wrong_filler_and_borrowers.push(user));

        // with wrong users it's just no-op
        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            redeemed_collateral,
        } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut wrong_filler_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();
        assert_eq!(redeemed_stablecoin, 0);
        assert!(redeemed_collateral.clearer.is_zero());
        assert!(redeemed_collateral.filler.is_zero());
        assert!(redeemed_collateral.redeemer.is_zero());
        assert!(redeemed_collateral.stakers.is_zero());

        // Clear empty is noop
        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            redeemed_collateral,
        } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut vec![],
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();
        assert_eq!(redeemed_stablecoin, 0);
        assert!(redeemed_collateral.filler.is_zero());
        assert!(redeemed_collateral.clearer.is_zero());
        assert!(redeemed_collateral.stakers.is_zero());
        assert!(redeemed_collateral.redeemer.is_zero());

        // Clear partially works and sets it to open
        let mut fillers_and_borrowers: Vec<&mut UserMetadata> = vec![&mut fill_bot];
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            ..
        } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();

        utils::print_candidate_users(redemptions_queue.clone(), 0);
        utils::print_order("After clearing", redemptions_queue.clone(), 0);

        let expected_burned_amount = borrowers_snapshot[0].borrowed_stablecoin;
        utils::assert_debt_burned(
            expected_burned_amount,
            redeemed_stablecoin,
            &borrowers_snapshot,
            &borrowers,
        );

        // assert order is not in Clearing mode anymore
        utils::assert_order_open(redemptions_queue.clone(), 0);
        let mut borrowers_mut: Vec<&mut UserMetadata> = borrowers.iter_mut().map(|x| x).collect();

        // trying to fill with empty candidates is noop
        let res = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers_mut,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_redemption_cannot_add_more_than_minted_or_less_than_min() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 10;
        let borrow_per_user = 200.0;
        let too_much_requested_redeemption_amount = 2500.0;
        let too_little_requested_redeemption_amount = 1999.0;
        let now_timestamp = 0;

        let _ = new_borrowing_users_with_sol_collateral(
            count,
            vec![10.0; count],
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        let mut redeemer = new_approved_user(&mut market);
        let res = redemption_operations::add_redemption_order(
            &mut redeemer,
            &mut redemptions_queue.borrow_mut(),
            &mut market,
            &prices,
            now_timestamp,
            USDH::from(too_much_requested_redeemption_amount),
        );

        assert_eq!(res.err().unwrap(), BorrowError::CannotRedeemMoreThanMinted);

        let res = redemption_operations::add_redemption_order(
            &mut redeemer,
            &mut redemptions_queue.borrow_mut(),
            &mut market,
            &prices,
            now_timestamp,
            USDH::from(too_little_requested_redeemption_amount),
        );

        assert_eq!(res.err().unwrap(), BorrowError::RedemptionsAmountTooSmall);
    }

    #[ignore]
    #[test]
    fn test_redemption_wrong_order_wrong_fillers() {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 99;
        let borrow_per_user = 1000.0;
        let requested_redeemption_amount = USDH::from(6000.0);
        let now_timestamp = 0;

        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).map(|i| ((i + 1) as f64) * 100.0).collect(),
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        let mut orders: Vec<RedemptionOrderInfo> = (0..5)
            .map(|_| {
                utils::set_up_redemption_order(
                    &mut market,
                    &mut redemptions_queue.borrow_mut(),
                    &prices,
                    requested_redeemption_amount,
                    now_timestamp,
                )
            })
            .collect();

        // Try to fill order_two first, should err
        let res = utils::fill_redemption_order(
            &orders[1],
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::All,
            now_timestamp,
        )
        .err()
        .unwrap();
        assert_eq!(res, BorrowError::InvalidRedemptionOrder);

        let mut clearer = utils::new_approved_user(&mut market);

        // Try to clear order_two first, should err
        let mut borrowers_mut: Vec<&mut UserMetadata> = borrowers.iter_mut().map(|x| x).collect();

        let res = redemption_operations::clear_redemption_order(
            orders[1].order_id,
            &mut orders[1].redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers_mut,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .err()
        .unwrap();
        assert_eq!(res, BorrowError::InvalidRedemptionOrder);

        // Fill order one, clear it

        let mut filler_one = utils::fill_redemption_order(
            &orders[0],
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::All,
            now_timestamp,
        )
        .unwrap();

        let mut fillers_and_borrowers: Vec<&mut UserMetadata> = vec![];
        fillers_and_borrowers.push(&mut filler_one);
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        let ClearRedemptionOrderEffects { .. } = redemption_operations::clear_redemption_order(
            orders[0].order_id,
            &mut orders[0].redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();
        utils::assert_order_cleared(redemptions_queue.clone(), 0);

        // Try to fill/clear anything other than order 1 is err
        [0, 2, 3, 4].iter().for_each(|order_id| {
            let res = utils::fill_redemption_order(
                &orders[*order_id as usize],
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                &mut borrowers,
                utils::BorrowersFilter::All,
                now_timestamp,
            )
            .err()
            .unwrap();
            assert_eq!(res, BorrowError::InvalidRedemptionOrder);
        });

        [1, 2, 3, 4].iter().for_each(|order_id| {
            let mut filler = utils::fill_redemption_order(
                &orders[*order_id as usize],
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                &mut borrowers,
                utils::BorrowersFilter::All,
                now_timestamp,
            )
            .unwrap();
            // println!("After fill order id {}", order_id);
            utils::print_candidate_users(redemptions_queue.clone(), *order_id as usize);

            // Fill with wrong bot, err
            let mut filler_one_wrong_and_borrowers: Vec<&mut UserMetadata> = vec![];
            filler_one_wrong_and_borrowers.push(&mut filler_one);
            borrowers
                .iter_mut()
                .for_each(|user| filler_one_wrong_and_borrowers.push(user));

            let err = redemption_operations::clear_redemption_order(
                orders[*order_id as usize].order_id,
                &mut orders[*order_id as usize].redeemer,
                &mut clearer,
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                &mut filler_one_wrong_and_borrowers,
                now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
            )
            .err()
            .unwrap();
            assert_eq!(err, BorrowError::RedemptionFillerNotFound);

            // Fill with correct bot, err
            let mut filler_one_correct_and_borrowers: Vec<&mut UserMetadata> = vec![];
            filler_one_correct_and_borrowers.push(&mut filler);
            borrowers
                .iter_mut()
                .for_each(|user| filler_one_correct_and_borrowers.push(user));

            let ClearRedemptionOrderEffects { .. } = redemption_operations::clear_redemption_order(
                orders[*order_id as usize].order_id,
                &mut orders[*order_id as usize].redeemer,
                &mut clearer,
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                &mut filler_one_correct_and_borrowers,
                now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
            )
            .unwrap();

            utils::print_candidate_users(redemptions_queue.clone(), *order_id as usize);
            utils::assert_order_cleared(redemptions_queue.clone(), *order_id as usize);
        });

        // 100499998 * 5 = 502499990
        // 600000000 / 100499998 = 5.970149372540286
        // 5.9 users, get 5 of them fully, one 90%
        // 100499998 * 5 = 502499990 - 600000000 = -97500010 + 100499998 = 2999988
        // (0..30 as usize).for_each(|i| {
        //     println!("{:?}", borrowers[i].borrowed_stablecoin);
        // });

        (0..30 as usize).for_each(|i| {
            // first 5 users for every batch are fully redeemed
            // every 6th user is only partially redeemed
            // and then, because there is a new order coming through
            // the 6th user has already been redeemed and has a much better CR
            // so it gets to the end of the potential queue

            if (i + 1) % 6 == 0 {
                assert_eq!(borrowers[i].borrowed_stablecoin, 30000000);
            } else {
                assert_eq!(borrowers[i].borrowed_stablecoin, 0);
            }
        });

        utils::assert_queue_is_empty(redemptions_queue.clone());
    }

    #[test]
    fn test_redemption_wrong_users() {
        // - [x] filling again with already redeemed users should fail (noop since infinite CR)
        //     - [x] should not be allowed to redeem users with no debt
        // - [x] test 10 users submitted, 5 users cleared, 5 users cleared again, not more not less

        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 99;
        let borrow_per_user = 1000.0;
        let requested_redeemption_amount = USDH::from(6000.0);
        let now_timestamp = 0;

        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).map(|i| ((i + 1) as f64) * 100.0).collect(),
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        let mut order = utils::set_up_redemption_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            requested_redeemption_amount,
            now_timestamp,
        );

        let mut clearer = utils::new_approved_user(&mut market);
        utils::print_candidate_users(redemptions_queue.clone(), 0);
        println!("0");

        // Fill order one first 3 users
        let mut fill_bots_one = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        )
        .unwrap();

        let mut filler_one_and_borrowers: Vec<&mut UserMetadata> = vec![];
        filler_one_and_borrowers.push(&mut fill_bots_one);
        borrowers
            .iter_mut()
            .for_each(|user| filler_one_and_borrowers.push(user));

        utils::print_candidate_users(redemptions_queue.clone(), 0);

        println!("c");
        let ClearRedemptionOrderEffects { .. } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut filler_one_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();
        utils::print_candidate_users(redemptions_queue.clone(), 0);
        utils::assert_pending_active_users(redemptions_queue.clone(), 0, 0);
        utils::assert_order_open(redemptions_queue.clone(), 0);

        println!("d");
        // Fill order one again with teh same first 3 users
        let _ = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        )
        .unwrap();
        utils::assert_pending_active_users(redemptions_queue.clone(), 0, 0);
        utils::print_candidate_users(redemptions_queue.clone(), 0);

        // Fill the rest of the users
        println!("Filling with 3, 4, 5");
        let mut fill_bot_two = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![3, 4, 5]),
            now_timestamp,
        )
        .unwrap();
        println!("Filled with 3, 4, 5");
        utils::print_candidate_users(redemptions_queue.clone(), 0);

        println!(
            "Clearing order {}",
            redemptions_queue.borrow().orders[0].to_state_string()
        );
        utils::assert_order_status(redemptions_queue.clone(), 0, RedemptionOrderStatus::Filling);
        println!("a");

        let mut filler_two_and_borrowers: Vec<&mut UserMetadata> = vec![];
        filler_two_and_borrowers.push(&mut fill_bot_two);
        borrowers
            .iter_mut()
            .for_each(|user| filler_two_and_borrowers.push(user));

        let ClearRedemptionOrderEffects { .. } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut filler_two_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();

        utils::assert_pending_active_users(redemptions_queue.clone(), 0, 0);
        utils::print_candidate_users(redemptions_queue.clone(), 0);

        utils::assert_order_cleared(redemptions_queue.clone(), 0);
        utils::assert_queue_is_empty(redemptions_queue.clone());
    }

    #[test]
    fn test_redemption_multiple_or_wrong_clearers() {
        // - [x] filling again with already redeemed users should fail (noop since infinite CR)
        // - [x] should not be allowed to redeem users with no debt
        // - [x] test 10 users submitted, 5 users cleared, 5 users cleared again, not more not less

        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        // 0 has the lowest CR, 99 has the highest CR
        let count = 99;
        let borrow_per_user = 1000.0;
        let requested_redeemption_amount = USDH::from(6000.0);
        let now_timestamp = 0;
        let collaterals = (0..count).map(|i| ((i + 1) as f64) * 100.0).collect();

        let (mut borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            collaterals,
            &mut market,
            &mut staking_pool_state,
            borrow_per_user,
            now_timestamp,
        );

        // for test assertions
        let borrowers_snapshot = borrowers.clone();

        let mut order = utils::set_up_redemption_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            requested_redeemption_amount,
            now_timestamp,
        );

        let mut clearer = utils::new_approved_user(&mut market);

        let mut good_filler_one = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![0, 1, 2]),
            now_timestamp,
        )
        .unwrap();

        let mut good_filler_two = utils::fill_redemption_order(
            &order,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            utils::BorrowersFilter::Some(vec![3, 4, 5]),
            now_timestamp,
        )
        .unwrap();

        // Create & clear with two random fillers
        let mut wrong_filler_one = utils::new_approved_user(&mut market);
        let mut wrong_filler_two = utils::new_approved_user(&mut market);

        let mut wrong_fillers_and_borrowers: Vec<&mut UserMetadata> = vec![];
        wrong_fillers_and_borrowers.push(&mut wrong_filler_one);
        wrong_fillers_and_borrowers.push(&mut wrong_filler_two);

        borrowers
            .iter_mut()
            .for_each(|user| wrong_fillers_and_borrowers.push(user));

        let res = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut wrong_fillers_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .err()
        .unwrap();
        assert_eq!(res, BorrowError::RedemptionFillerNotFound);

        // Clear with the wrong users
        // no error, but should just be noop
        let mut good_fillers_and_wrong_borrowers: Vec<&mut UserMetadata> = vec![];
        good_fillers_and_wrong_borrowers.push(&mut good_filler_one);
        good_fillers_and_wrong_borrowers.push(&mut good_filler_two);

        borrowers[3..]
            .iter_mut()
            .for_each(|user| good_fillers_and_wrong_borrowers.push(user));

        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            redeemed_collateral,
        } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut good_fillers_and_wrong_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();
        assert_eq!(redeemed_stablecoin, 0);
        assert!(redeemed_collateral.stakers.is_zero());
        assert!(redeemed_collateral.redeemer.is_zero());
        assert!(redeemed_collateral.filler.is_zero());
        assert!(redeemed_collateral.clearer.is_zero());

        let mut good_fillers_and_good_borrowers: Vec<&mut UserMetadata> = vec![];
        good_fillers_and_good_borrowers.push(&mut good_filler_one);
        good_fillers_and_good_borrowers.push(&mut good_filler_two);

        borrowers
            .iter_mut()
            .for_each(|user| good_fillers_and_good_borrowers.push(user));

        let ClearRedemptionOrderEffects { .. } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut good_fillers_and_good_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )
        .unwrap();

        // Expected values for first 3 bots
        // Expected values for last 3 bots
        let collateral_infos: Vec<CollateralInfo> = borrowers_snapshot
            .iter()
            .map(|borrower| CollateralInfo::from(borrower, &prices))
            .collect();

        let (_, fill_bots_values) = borrowers_snapshot[0..6]
            .iter()
            .zip(collateral_infos.iter())
            .enumerate()
            .fold(
                (requested_redeemption_amount, vec![]),
                |(remaining, mut fill_bots_values), (_i, (borrower, coll_info))| {
                    let redeemed_amount = u64::min(borrower.borrowed_stablecoin, remaining);
                    fill_bots_values.push(
                        borrower
                            .deposited_collateral
                            .mul_fraction(redeemed_amount, coll_info.collateral_value)
                            .mul_bps(5),
                    );
                    (remaining - redeemed_amount, fill_bots_values)
                },
            );

        let first_bot_expected = fill_bots_values[0..3]
            .iter()
            .fold(CollateralAmounts::default(), |acc, fill_bot| {
                acc.add(&fill_bot)
            });
        let second_bot_expected = fill_bots_values[3..6]
            .iter()
            .fold(CollateralAmounts::default(), |acc, fill_bot| {
                acc.add(&fill_bot)
            });

        assert_eq!(good_filler_one.inactive_collateral, first_bot_expected);
        assert_eq!(good_filler_two.inactive_collateral, second_bot_expected);

        utils::print_candidate_users(redemptions_queue.clone(), 0);
        utils::assert_pending_active_users(redemptions_queue.clone(), 0, 0);

        utils::assert_order_cleared(redemptions_queue.clone(), 0);
        utils::assert_queue_is_empty(redemptions_queue.clone());
    }

    #[test]
    fn test_redemption_cannot_redeem_user_under_mcr() {
        // - [x] redemption: ensure when redeeming the user is well collateralized
        // - [x] redemption: // Find the first trove with ICR >= MCR

        // Everyone deposits 1000 SOL
        // Everyone has a debt of 1000.0 USDH
        // SOL/USD price is 1.09
        // Such that everyone is at 109% CR
        // But there is a whale that keeps the system well collateralized

        let prices_at_beginning = 2.0;
        let prices_at_redemption = 1.09;
        let now_timestamp = 0;
        let whale_deposit = SOL::from(1000.0 * 100.0);
        let users_deposit = SOL::from(1000.0);
        let users_deposit = CollateralAmounts::of_token(users_deposit, CollateralToken::SOL);
        let borrow_amt = USDH::from(1000.0);
        let redeem_amt = USDH::from(2000.0);
        let num = 100;
        let collaterals: Vec<CollateralAmounts> = vec![users_deposit; num];

        let (mut market, mut spool, redemptions_queue, prices) =
            setup_redemption_borrowing_program_with_prices(prices_at_beginning);
        let borrow_splits = vec![borrow_amt; num];

        let _whale = new_borrower(
            &mut market,
            &mut spool,
            whale_deposit,
            borrow_amt,
            &prices,
            now_timestamp,
        );

        let mut borrowers = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut spool,
            num,
            &borrow_splits,
            &collaterals,
            prices_at_beginning,
            now_timestamp,
        );

        let FilledOrderSetUp { .. } = set_up_filled_redemption_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            BorrowersFilter::All,
            &TokenPrices::new(prices_at_redemption),
            redeem_amt,
            now_timestamp,
        )
        .unwrap();
        utils::assert_num_active_candidates(redemptions_queue.clone(), 0, 0);
    }

    #[test]
    fn test_redemption_cannot_redeem_tcr_under_mcr() {
        // - [X] assert redemption constraints - cannot redeem TCR < MCR _requireTCRoverMCR
        // - [x] redemption: ensure when redeeming the user is well collateralized
        // - [x] redemption: // Find the first trove with ICR >= MCR

        // Everyone deposits 1000 SOL
        // Everyone has a debt of 1000.0 USDH
        // SOL/USD price is 1.09
        // Such that everyone is at 109% CR

        let prices_at_beginning = 2.0;
        let prices_at_redemption = 1.09;
        let now_timestamp = 0;
        let users_deposit = SOL::from(1000.0);
        let users_deposit = CollateralAmounts::of_token(users_deposit, CollateralToken::SOL);
        let borrow_amt = USDH::from(1000.0);
        let redeem_amt = USDH::from(2000.0);
        let num = 100;
        let collaterals: Vec<CollateralAmounts> = vec![users_deposit; num];

        let (mut market, mut spool, redemptions_queue, _prices) =
            setup_redemption_borrowing_program_with_prices(prices_at_beginning);
        let borrow_amounts = vec![borrow_amt; num];

        let _ = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut spool,
            num,
            &borrow_amounts,
            &collaterals,
            prices_at_beginning,
            now_timestamp,
        );

        let mut redeemer = new_approved_user(&mut market);
        let res = redemption_operations::add_redemption_order(
            &mut redeemer,
            &mut redemptions_queue.borrow_mut(),
            &mut market,
            &TokenPrices::new(prices_at_redemption),
            now_timestamp,
            redeem_amt,
        );

        assert_eq!(
            res.err().unwrap(),
            crate::BorrowError::CannotRedeemWhenUndercollateralized
        );
    }

    #[test]
    fn test_redemption_cannot_redeem_before_bootstrapping_period() {
        let prices_at_beginning = 2.0;
        let now_timestamp = 2;
        let users_deposit = SOL::from(1000.0);
        let users_deposit = CollateralAmounts::of_token(users_deposit, CollateralToken::SOL);
        let borrow_amt = USDH::from(1000.0);
        let redeem_amt = USDH::from(2000.0);
        let num = 100;
        let collaterals: Vec<CollateralAmounts> = vec![users_deposit; num];

        let mut market = BorrowingMarketState::new();
        let mut spool = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let redemptions_queue = RefCell::new(RedemptionsQueue::default());
        let prices = TokenPrices::new(prices_at_beginning);
        borrowing_operations::initialize_borrowing_market(
            &mut market,
            now_timestamp + BOOTSTRAP_PERIOD,
        );

        let borrow_amounts = vec![borrow_amt; num];

        let _ = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut spool,
            num,
            &borrow_amounts,
            &collaterals,
            prices_at_beginning,
            now_timestamp,
        );

        let mut redeemer = new_approved_user(&mut market);
        let res = redemption_operations::add_redemption_order(
            &mut redeemer,
            &mut redemptions_queue.borrow_mut(),
            &mut market,
            &prices,
            now_timestamp - 1,
            redeem_amt,
        );

        assert_eq!(
            res.err().unwrap(),
            crate::BorrowError::CannotRedeemDuringBootstrapPeriod
        );
    }

    #[test]
    fn test_redemption_cannot_redeem_more_than_minted() {
        let prices_at_beginning = 2.0;
        let now_timestamp = 0;
        let redeem_amt = USDH::from(2000.0);

        let mut market = BorrowingMarketState::new();
        let redemptions_queue = RefCell::new(RedemptionsQueue::default());
        let prices = TokenPrices::new(prices_at_beginning);
        borrowing_operations::initialize_borrowing_market(
            &mut market,
            now_timestamp + BOOTSTRAP_PERIOD,
        );

        let mut redeemer = new_approved_user(&mut market);
        let res = redemption_operations::add_redemption_order(
            &mut redeemer,
            &mut redemptions_queue.borrow_mut(),
            &mut market,
            &prices,
            now_timestamp,
            redeem_amt,
        );

        assert_eq!(
            res.err().unwrap(),
            crate::BorrowError::CannotRedeemMoreThanMinted
        );
    }

    #[test]
    fn test_redemption_cannot_redeem_more_than_minted_subsequent_redemptions_full_depletion() {
        // TODO: shall we make pending redemption amount
        // as inactive debt?

        let prices = 2.0;
        let now_timestamp = 0;
        let whale_deposit = SOL::from(100000.0);
        let whale_borrow = USDH::from(2000.0 * 1.5);
        let redeem_amt = USDH::from(2000.0);

        let (mut market, mut spool, redemptions_queue, prices) =
            setup_redemption_borrowing_program_with_prices(prices);

        let _whale = new_borrower(
            &mut market,
            &mut spool,
            whale_deposit,
            whale_borrow,
            &prices,
            now_timestamp,
        );

        let mut redeemer = new_approved_user(&mut market);

        // Redeeming once works, 500 is left
        let res = redemption_operations::add_redemption_order(
            &mut redeemer,
            &mut redemptions_queue.borrow_mut(),
            &mut market,
            &prices,
            now_timestamp,
            redeem_amt,
        );
        println!("Res {:?}", res);
        assert!(res.is_ok());

        // This is trying to redeem 1000.0, shouldn't be allowed
        let res = redemption_operations::add_redemption_order(
            &mut redeemer,
            &mut redemptions_queue.borrow_mut(),
            &mut market,
            &prices,
            now_timestamp,
            redeem_amt,
        );
        assert_eq!(
            res.err().unwrap(),
            crate::BorrowError::CannotRedeemMoreThanMinted
        );
    }

    #[test]
    fn test_redemption_assert_user_stake_and_total_stakes_updated() {
        // - [x] redemption: _updateStakeAndTotalStakes
        // Everyone deposits 1000 SOL
        // Everyone has a debt of 1000.0 USDH
        // SOL/USD price is 1.09
        // Such that everyone is at 109% CR
        // But there is a whale that keeps the system well collateralized
        // We redeem 2000 from the first two users
        // one with 1005 USDH and one with 1005 USDH
        // the first one remains with 0, the second one remains with 1005 - (2000 - 1005) = 10
        // expecting total system stake to reduce by 2000
        // and user stake to reduce by 1005 and 995

        let prices = 2.0;

        let (mut market, mut spool, redemptions_queue, _prices) =
            setup_redemption_borrowing_program_with_prices(prices);

        let now_timestamp = 0;
        let whale_deposit = SOL::from(1000.0 * 100.0);
        let borrow_amt = USDH::from(1000.0);
        let num = 100;
        let collaterals: Vec<CollateralAmounts> = (0..num)
            .map(|i| {
                CollateralAmounts::of_token(
                    SOL::from(((i + 1) as f64) * 1000.0),
                    CollateralToken::SOL,
                )
            })
            .collect();
        let redeem_amt = USDH::from(2000.0);
        let borrow_split = BorrowSplit::from_amount(borrow_amt, market.base_rate_bps);
        let borrow_splits = vec![borrow_amt; num];

        let (_whale, _) = new_borrower(
            &mut market,
            &mut spool,
            whale_deposit,
            borrow_amt,
            &TokenPrices::new(prices),
            now_timestamp,
        );

        assert_eq!(_whale.user_stake, borrow_split.amount_to_borrow);

        let mut borrowers = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut spool,
            num,
            &borrow_splits,
            &collaterals,
            prices,
            now_timestamp,
        );
        borrowers
            .iter()
            .for_each(|u| assert_eq!(u.user_stake, borrow_split.amount_to_borrow));

        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(prices),
            redeem_amt,
            now_timestamp,
        )
        .unwrap();

        assert_eq!(borrowers[0].user_stake, 0);
        assert_eq!(borrowers[1].user_stake, USDH::from(10.0));
        assert_eq!(
            market.total_stake,
            borrow_split.amount_to_borrow * ((num + 1) as u64) - redeem_amt // including whale
        );
    }

    #[test]
    fn test_redemption_assert_pending_rewards_applied() {
        // There is a liquidation event, users get redistributed an amount
        // then there is a redemption event
        // and users get redeemed including the redistributed amount

        // 1 borrower whale with 1000 usdh & 100000000 sol
        // 3 borrowers with 1000 usdh & 1000, 2000, 3000 sol
        // stability pool is empty
        // borrower[0] gets liquidated
        // the other two guys should receive

        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let redemptions_queue = RefCell::new(RedemptionsQueue::default());

        let prices_at_beginning = 2.0;
        let prices_at_liquidation = 1.09;
        let prices_at_redemption = 1.0;
        let whale_deposit = SOL::from(100000.0);
        let redeem_amt = USDH::from(2000.0);
        let now_timestamp = 0;
        let borrow_amt = USDH::from(1000.0);
        let num = 3;
        let collaterals: Vec<CollateralAmounts> = [1000.0, 2000.0, 3000.0]
            .iter()
            .map(|amt| CollateralAmounts::of_token(SOL::from(*amt), CollateralToken::SOL))
            .collect();
        let borrow_split = BorrowSplit::from_amount(borrow_amt, market.base_rate_bps);
        let borrow_amounts = vec![borrow_amt; num];

        let (mut whale, _) = new_borrower(
            &mut market,
            &mut staking_pool_state,
            whale_deposit,
            borrow_amt,
            &TokenPrices::new(prices_at_beginning),
            now_timestamp,
        );

        let mut borrowers = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            num,
            &borrow_amounts,
            &collaterals,
            prices_at_beginning,
            now_timestamp,
        );

        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowers[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(prices_at_liquidation),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        borrowing_operations::refresh_positions(&mut market, &mut borrowers[0]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut borrowers[1]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut borrowers[2]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut whale).unwrap();

        // TODO: think about inactive collateral & redsitributeion & total stake

        // Liquidating 1005 USDH and 1000 SOL ->
        // 1005 / 3  = 335
        // 1000 - 0.005% (for liquidator) = 995 / 3 = 331.6666666666667

        // borrowers have 1005 + 335 = 1340 (1339.999)

        // borrower 0 has 0 debt 0 coll
        // borrower 1 has 1339999999 debt 2331666666666 coll
        // borrower 2 has 1339999999 debt 3331666666666 coll
        // whale not included

        // redeeming 2000 -> 2000000000
        // expecting borrower 1 to have 0 debt left 2331666666666 - 1339999999000 = 991666667666 coll
        // 2000000000 - 1339999999 = 660000001 1339999999 - 660000001 = 679999998
        // expecting borrower 1 to have 679999998 debt left 3331666666666 - 660000001000 = 2671666665666 coll

        let debt_redistributed = borrow_split.amount_to_borrow / 3;
        let coll_redistributed = collaterals[0].sol * 9995 / 10000 / 3;

        println!("{} {}", debt_redistributed, coll_redistributed);

        let (_order, _clearer, _fill_bot, _redeemed_stablecoin, _redeemed_collateral) =
            add_fill_and_clear_order(
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                &mut borrowers,
                &TokenPrices::new(prices_at_redemption),
                redeem_amt,
                now_timestamp,
            )
            .unwrap();

        // borrower 1 had 1005 USDH + 335 (redistributed) and loses all to redemption
        // borrower 1 had 1005 + 335 - (2000 - 1340) = 680
        assert_eq!(borrowers[0].borrowed_stablecoin, 0);
        assert_eq!(borrowers[0].user_stake, 0);
        assert_eq!(borrowers[1].borrowed_stablecoin, 0);
        assert_eq!(borrowers[1].user_stake, 0);

        let borrowers_after_liq_before_redemption =
            borrow_split.amount_to_borrow + debt_redistributed;
        let _collateral_after_liq_before_redemption = collaterals[2].sol + coll_redistributed;

        let _reeemed_from_first_user = borrowers_after_liq_before_redemption;
        assert_fuzzy_eq!(
            borrowers[2].borrowed_stablecoin,
            borrowers_after_liq_before_redemption
                - (redeem_amt - borrowers_after_liq_before_redemption),
            3
        );
        assert_fuzzy_eq!(
            borrowers[2].deposited_collateral.sol,
            2671666665666 as u64,
            132
        );
    }

    #[test]
    fn test_redemption_assert_pending_rewards_applied_after_redemption_and_redistribution() {
        // 1 borrower whale with 1000 usdh & 100000000 sol
        // 4borrowers with 1000 usdh & 1000, 2000, 3000, 4000 sol
        // stability pool is empty
        // borrower[0] gets liquidated
        // the other two guys should receive

        // User goes through these events
        // 1. Redistribution - borrower 0 gets liquidated
        // 2. Redemption - borrower 1 gets redeemed fully, borrower 2 gets redeemed partially
        // 3. Redistribution - borrower 4 gets liquidated
        // 4. Borrower 3 - Withdraw USDH
        // Assert the pending rewards have been updated after all steps correctly

        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };
        let redemptions_queue = RefCell::new(RedemptionsQueue::default());

        let prices_at_beginning = 2.0;
        let prices_at_liquidation = 1.09;
        let prices_at_liquidation_2 = 0.32;
        let prices_at_redemption = 1.0;
        let whale_deposit = SOL::from(100000.0);
        let redeem_amt = USDH::from(2000.0);
        let now_timestamp = 0;
        let borrow_amt = USDH::from(1000.0);
        let num = 4;
        let collaterals: Vec<CollateralAmounts> = [1000.0, 2000.0, 3000.0, 4000.0]
            .iter()
            .map(|amt| CollateralAmounts::of_token(SOL::from(*amt), CollateralToken::SOL))
            .collect();
        let _borrow_split = BorrowSplit::from_amount(borrow_amt, market.base_rate_bps);
        let borrow_amounts = vec![borrow_amt; num];

        let (mut whale, _) = new_borrower(
            &mut market,
            &mut staking_pool_state,
            whale_deposit,
            borrow_amt,
            &TokenPrices::new(prices_at_beginning),
            now_timestamp,
        );

        let mut borrowers = new_borrowing_users_with_amounts_and_price(
            &mut market,
            &mut staking_pool_state,
            num,
            &borrow_amounts,
            &collaterals,
            prices_at_beginning,
            now_timestamp,
        );

        // Liquidate user 0 - Redistribution
        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowers[0],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(prices_at_liquidation),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        // Redeem users 1 fully and 2 partially - Redemption
        let _ = add_fill_and_clear_order(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &mut borrowers,
            &TokenPrices::new(prices_at_redemption),
            redeem_amt,
            now_timestamp,
        )
        .unwrap();

        // Liquidate user 3 - Redistribution
        let liquidator = Pubkey::new_unique();
        borrowing_operations::try_liquidate(
            liquidator,
            &mut market,
            &mut borrowers[3],
            &mut stability_pool_state,
            &mut epoch_to_scale_to_sum,
            &TokenPrices::new(prices_at_liquidation_2),
            &mut liquidations.borrow_mut(),
            now_timestamp,
        )
        .unwrap();

        // T=0
        // w debt=1005                      coll=100000
        // 0 debt=1005                      coll=1000
        // 1 debt=1005                      coll=2000
        // 2 debt=1005                      coll=3000
        // 3 debt=1005                      coll=4000

        // After redistribution due to user 0's liquidation
        // debt redis 1005 / 4 = 251.25
        // coll redis 995 / 4 = 248.75
        // w debt=1005+251.25=1256.25       coll=100000+248.75=100248.75
        // 0 debt=0                         coll=0
        // 1 debt=1005+251.25=1256.25       coll=2000+248.75=2248.75
        // 2 debt=1005+251.25=1256.25       coll=3000+248.75=3248.75
        // 3 debt=1005+251.25=1256.25       coll=4000+248.75=4248.75

        // After redemption of 2000 from users 1 and 2
        // redeeming at price=1.0
        // user 1 gets redeemed 1256.25 debt & coll
        // user 2 gets redeemed 2000 - 1256.25 = 743.75 debt & coll
        // w debt=1256.25                   coll=100248.75
        // 0 debt=0                         coll=0
        // 1 debt=0                         coll=2248.75-1256.25=992.5
        // 2 debt=1256.25-743.75 = 512.5    coll=3248.75-743.75=2505
        // 3 debt=1256.25                   coll=4248.75

        // 103251325110423 - 100250000000000 = 3001325110423
        // 100250000000000

        // 247512437810945

        // 3001.325110423 / 0.7102473498233216 = 4225.746299749795
        // 2148498233 * 2390109540636607 / 1000000000 = 5135146124734192

        // 4227506250001 / 1768750000 = 2390.1095406366076
        // 2390 * 1768750000 = 4227312500000
        // 4227.312500000 * 0.7102473498233216 = 3002.4375 + 100250 = 103252.4375
        // 4227.312500000 * 0.28975265017667845 = 1224.875

        // Liquidating user3 at
        // if px = 1.09 -> 4248.75 * 1.09 / 1256.25  ICR=3.686477611940299
        // 4248.75 / 1.09 = 1256.25 / 3897.9357798165133  = 0.32228596646072377
        // if px = 0.32 -> 4248.75 * 0.32 / 1256.25 ICR=1.082268656716418
        // debt redis = 1256.25
        // coll redis = 4248.75 * 0.995 = 4227.50625
        // total debt excluding user = 1256.25 + 512.5 = 1768.75
        // w represents 1256.25 / 1768.75 = 0.7102473498233216
        // 2 represents 512.5 / 1768.75 = 0.28975265017667845
        // debt to whale 0.7102473498233216 * 1256.25 = 892.2482332155478
        // debt to 2 0.28975265017667845 * 1256.25 = 364.0017667844523
        // coll to whale 0.7102473498233216 * 4227.50625 = 3002.575110424029
        // coll to 2 0.28975265017667845 * 4227.50625 = 1224.9311395759719
        // w debt=1256.25+892.2482332155478 = 2148.4982332155478 coll=100248.75+3002.575110424029 = 103251.32511042403
        // 0 debt=0                         coll=0
        // 1 debt=0                         coll=0
        // 2 debt=512.5+364.0017667844523 = 876.5017667844522  coll=2505+1224.9311395759719 = 3729.931139575972
        // 3 debt=0                         coll=0

        // market 876.5017667844522 + 2148.4982332155478 = 3025

        borrowing_operations::refresh_positions(&mut market, &mut borrowers[0]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut borrowers[1]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut borrowers[2]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut borrowers[3]).unwrap();
        borrowing_operations::refresh_positions(&mut market, &mut whale).unwrap();

        // Debt
        assert_eq!(whale.borrowed_stablecoin, USDH::from(2148.4982332155478));
        assert_eq!(borrowers[0].borrowed_stablecoin, 0);
        assert_eq!(borrowers[1].borrowed_stablecoin, 0);
        assert_eq!(borrowers[2].borrowed_stablecoin, USDH::from(876.50176678));
        assert_eq!(borrowers[3].borrowed_stablecoin, 0);
        assert_eq!(market.stablecoin_borrowed, USDH::from(3025.0));

        // Collateral
        // There is precision loss here due to
        // redistrib_coll / total_stake
        assert_eq!(whale.deposited_collateral.sol, SOL::from(103251.325110424));
        assert_eq!(borrowers[0].deposited_collateral.sol, 0);
        assert_eq!(borrowers[1].deposited_collateral.sol, 0);
        assert_fuzzy_eq!(
            borrowers[2].deposited_collateral.sol,
            SOL::from(3729.931139575972),
            300
        );
        assert_eq!(borrowers[3].deposited_collateral.sol, 0);
    }
}

#[cfg(test)]
mod property_tests {

    use crate::redemption::test_redemptions::tests::RedemptionOrderInfo;
    use crate::redemption::test_redemptions::utils::{
        fill_redemption_order_new_fillers, new_borrowing_users_with_sol_collateral,
        new_redemption_orders, setup_redemption_borrowing_program,
    };
    use crate::state::CandidateRedemptionUser;
    use crate::utils::coretypes::USDH;
    use crate::UserMetadata;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use std::convert::TryInto;

    #[quickcheck]
    fn test_all_user_ids(user_ids: Vec<Vec<u8>>) -> bool {
        run_property_test(user_ids)
    }

    #[quickcheck]
    fn test_user_ids_in_range(user_ids: Vec<Vec<WithinRangeUser>>) -> bool {
        let vec: Vec<Vec<u8>> = user_ids
            .iter()
            .map(|chunk| chunk.iter().map(|within| within.0).collect())
            .collect();
        run_property_test(vec)
    }

    fn run_property_test(user_ids: Vec<Vec<u8>>) -> bool {
        let (mut market, mut staking_pool_state, redemptions_queue, prices) =
            setup_redemption_borrowing_program();

        let count = 31;
        let redeem_amt = USDH::from(2500.0);
        let now_timestamp = 0;
        let (borrowers, _) = new_borrowing_users_with_sol_collateral(
            count,
            (0..count).rev().map(|i| ((i + 1) as f64) * 100.0).collect(),
            &mut market,
            &mut staking_pool_state,
            3000.0,
            now_timestamp,
        );

        let [order_1, _order_2]: [RedemptionOrderInfo; 2] = new_redemption_orders(
            &mut market,
            &mut redemptions_queue.borrow_mut(),
            &prices,
            vec![redeem_amt, redeem_amt],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        let borrowers_of_ids = |ids: &Vec<u8>| -> Vec<UserMetadata> {
            borrowers
                .clone()
                .into_iter()
                .filter(|user| ids.contains(&(user.user_id as u8)))
                .collect()
        };

        let mut chunks: Vec<Vec<UserMetadata>> = user_ids
            .iter()
            .map(|chunk| borrowers_of_ids(chunk))
            .collect();

        for chunk in chunks.iter_mut() {
            let borrowers_mut: Vec<&mut UserMetadata> = chunk.iter_mut().map(|x| x).collect();
            let _bots: Vec<UserMetadata> = fill_redemption_order_new_fillers(
                &mut market,
                &mut redemptions_queue.borrow_mut(),
                order_1.order_id,
                vec![borrowers_mut],
                now_timestamp,
            );
        }

        let result_users: Vec<CandidateRedemptionUser> = redemptions_queue.borrow().orders[0]
            .candidate_users
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| candidate.clone())
            .collect();

        let mut expected_result: Vec<CandidateRedemptionUser> = result_users
            .iter()
            .map(|candidate| candidate.clone())
            .collect();
        expected_result
            .sort_by(|a, b| a.collateral_ratio.partial_cmp(&b.collateral_ratio).unwrap());

        result_users == expected_result
    }

    impl Arbitrary for WithinRangeUser {
        fn arbitrary(g: &mut Gen) -> Self {
            let vec: Vec<u8> = (0..100).collect();
            let option: &u8 = g.choose(vec.as_slice()).unwrap();
            WithinRangeUser(*option)
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct WithinRangeUser(u8);
}

#[cfg(test)]
pub(crate) mod utils {

    use std::convert::TryInto;
    use std::ops::SubAssign;
    use std::{borrow::BorrowMut, cell::RefMut};

    use crate::borrowing_market::borrowing_operations::utils::set_addresses;
    use crate::borrowing_market::borrowing_rate::BorrowSplit;
    use crate::redemption::redemption_operations;
    use crate::redemption::redemption_operations::calcs::split_redemption_collateral;
    use crate::redemption::types::{ClearRedemptionOrderEffects, RedemptionCollateralSplit};
    use crate::state::redemptions_queue::{RedemptionCandidateStatus, RedemptionOrderStatus};
    use crate::utils::consts::REDEMPTIONS_SECONDS_TO_FILL_ORDER;
    use crate::utils::coretypes::USDH;
    use crate::utils::finance::CollateralInfo;
    use crate::{
        borrowing_market::tests_utils::utils::new_borrowing_users_with_amounts,
        state::CollateralToken, BorrowingMarketState, CollateralAmounts, RedemptionsQueue,
        StakingPoolState, TokenPrices, UserMetadata,
    };

    use crate::borrowing_market::borrowing_operations;
    use anchor_lang::prelude::Pubkey;
    use anchor_lang::solana_program::native_token::sol_to_lamports;
    use core::cmp;
    use rand::prelude::SliceRandom;
    use rand::thread_rng;
    use std::cell::RefCell;

    use super::tests::RedemptionOrderInfo;

    pub fn setup_redemption_borrowing_program() -> (
        BorrowingMarketState,
        StakingPoolState,
        RefCell<RedemptionsQueue>,
        TokenPrices,
    ) {
        setup_redemption_borrowing_program_with_prices(40.0)
    }

    pub fn setup_redemption_borrowing_program_with_prices(
        price: f64,
    ) -> (
        BorrowingMarketState,
        StakingPoolState,
        RefCell<RedemptionsQueue>,
        TokenPrices,
    ) {
        let mut market = BorrowingMarketState::new();
        let staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let redemptions_queue = RefCell::new(RedemptionsQueue::default());
        let prices = TokenPrices::new(price);

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        (market, staking_pool_state, redemptions_queue, prices)
    }

    pub fn new_borrowing_users_with_sol_collateral(
        count: usize,
        sol_collaterals: Vec<f64>,
        market: &mut BorrowingMarketState,
        staking_pool_state: &mut StakingPoolState,
        borrow_amount: f64,
        now_timestamp: u64,
    ) -> (Vec<UserMetadata>, Vec<Pubkey>) {
        let collaterals: Vec<CollateralAmounts> = sol_collaterals
            .iter()
            .map(|amount_sol| {
                CollateralAmounts::of_token(sol_to_lamports(*amount_sol), CollateralToken::SOL)
            })
            .collect();

        let borrow_splits = vec![USDH::from(borrow_amount); count];
        let borrowers = new_borrowing_users_with_amounts(
            market,
            staking_pool_state,
            count,
            &borrow_splits,
            &collaterals,
            now_timestamp,
        );

        let borrowers_metadatas_pk: Vec<Pubkey> = (0..borrowers.len())
            .map(|i| borrowers[i].metadata_pk)
            .collect();

        (borrowers, borrowers_metadatas_pk)
    }

    pub fn new_redemption_order(
        redeemer: &mut UserMetadata,
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
        prices: &TokenPrices,
        market: &mut BorrowingMarketState,
        amount: u64,
        now_timestamp: u64,
    ) -> u64 {
        redemption_operations::add_redemption_order(
            redeemer,
            &mut redemptions_queue.borrow_mut(),
            market,
            prices,
            now_timestamp,
            amount,
        )
        .unwrap()
        .redemption_order_id
    }

    pub fn new_redemption_orders(
        market: &mut BorrowingMarketState,
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
        prices: &TokenPrices,
        amounts: Vec<u64>,
        now_timestamp: u64,
    ) -> Vec<RedemptionOrderInfo> {
        amounts
            .iter()
            .map(|amount| {
                let mut redeemer = new_approved_user(market);
                let order_id = new_redemption_order(
                    &mut redeemer,
                    &mut redemptions_queue.borrow_mut(),
                    prices,
                    market,
                    *amount,
                    now_timestamp,
                );
                RedemptionOrderInfo { redeemer, order_id }
            })
            .collect()
    }

    pub fn fill_redemption_order_new_bot(
        market: &mut BorrowingMarketState,
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
        order_id: u64,
        user_metadatas: &mut Vec<&mut UserMetadata>,
        now: u64,
    ) -> UserMetadata {
        let fill_bot = new_approved_user(market);

        let res = redemption_operations::fill_redemption_order(
            order_id,
            market,
            &mut redemptions_queue.borrow_mut(),
            user_metadatas,
            &fill_bot,
            now,
        );

        println!("Filled order {:?}", res);

        res.unwrap();

        fill_bot
    }

    pub fn fill_redemption_order_new_fillers(
        market: &mut BorrowingMarketState,
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
        order_id: u64,
        mut user_metadatas: Vec<Vec<&mut UserMetadata>>,
        now_timestamp: u64,
    ) -> Vec<UserMetadata> {
        user_metadatas
            .iter_mut()
            .map(|users| {
                fill_redemption_order_new_bot(
                    market,
                    redemptions_queue,
                    order_id,
                    users,
                    now_timestamp,
                )
            })
            .collect()
    }

    pub struct FilledOrderSetUp {
        pub order: RedemptionOrderInfo,
        pub fill_bot: UserMetadata,
    }

    #[derive(Debug, Clone)]
    pub enum BorrowersFilter {
        All,
        Some(Vec<u64>),
    }

    pub fn set_up_redemption_order(
        market: &mut BorrowingMarketState,
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
        prices: &TokenPrices,
        redeem_amount: u64,
        now_timestamp: u64,
    ) -> RedemptionOrderInfo {
        let [order_1]: [RedemptionOrderInfo; 1] = new_redemption_orders(
            market,
            redemptions_queue,
            prices,
            vec![redeem_amount],
            now_timestamp,
        )
        .try_into()
        .unwrap();

        order_1
    }

    pub fn fill_redemption_order(
        order: &RedemptionOrderInfo,
        market: &mut BorrowingMarketState,
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
        borrowers: &mut Vec<UserMetadata>,
        borrowers_filter: BorrowersFilter,
        now_timestamp: u64,
    ) -> Result<UserMetadata, crate::BorrowError> {
        let mut submitted_users: Vec<&mut UserMetadata> = borrowers
            .iter_mut()
            .filter(|u| match &borrowers_filter {
                BorrowersFilter::All => true,
                BorrowersFilter::Some(ids) => ids.contains(&u.user_id),
            })
            .collect();

        submitted_users.shuffle(&mut thread_rng());

        let fill_bot = new_approved_user(market);
        for users in submitted_users.chunks_mut(5) {
            redemption_operations::fill_redemption_order(
                order.order_id,
                market,
                redemptions_queue,
                users,
                &fill_bot,
                now_timestamp,
            )?;
        }

        Ok(fill_bot)
    }

    pub fn set_up_filled_redemption_order(
        market: &mut BorrowingMarketState,
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
        borrowers: &mut Vec<UserMetadata>,
        borrowers_filter: BorrowersFilter,
        prices: &TokenPrices,
        redeem_amount: u64,
        now_timestamp: u64,
    ) -> Result<FilledOrderSetUp, crate::BorrowError> {
        let order = set_up_redemption_order(
            market,
            redemptions_queue,
            prices,
            redeem_amount,
            now_timestamp,
        );

        println!("Redemption order {:?}", order);

        let fill_bot = fill_redemption_order(
            &order,
            market,
            redemptions_queue,
            borrowers,
            borrowers_filter,
            now_timestamp,
        )?;

        Ok(FilledOrderSetUp { order, fill_bot })
    }

    #[derive(Default, Debug)]
    pub struct RedemptionResults {
        pub fill_bot: CollateralAmounts,
        pub clear_bot: CollateralAmounts,
        pub stakers: CollateralAmounts,
        pub redeemer: CollateralAmounts,
        pub updated_borrowers: Vec<UserMetadata>,
        pub redeemed_amount: u64,
        pub remaining_amount: u64,
    }

    pub fn add_fill_and_clear_order(
        market: &mut BorrowingMarketState,
        redemptions_queue: &mut RefMut<RedemptionsQueue>,
        borrowers: &mut Vec<UserMetadata>,
        prices: &TokenPrices,
        redeem_amount: u64,
        now_timestamp: u64,
    ) -> Result<
        (
            RedemptionOrderInfo,
            UserMetadata,
            UserMetadata,
            u64,
            RedemptionCollateralSplit,
        ),
        crate::BorrowError,
    > {
        let FilledOrderSetUp {
            mut order,
            mut fill_bot,
        } = set_up_filled_redemption_order(
            market,
            redemptions_queue,
            borrowers,
            BorrowersFilter::All,
            prices,
            redeem_amount,
            now_timestamp,
        )?;

        let mut clearer = new_approved_user(market);
        let mut fillers_and_borrowers = vec![&mut fill_bot];
        borrowers
            .iter_mut()
            .for_each(|user| fillers_and_borrowers.push(user));

        let ClearRedemptionOrderEffects {
            redeemed_stablecoin,
            redeemed_collateral,
        } = redemption_operations::clear_redemption_order(
            order.order_id,
            &mut order.redeemer,
            &mut clearer,
            market,
            &mut redemptions_queue.borrow_mut(),
            &mut fillers_and_borrowers,
            now_timestamp + REDEMPTIONS_SECONDS_TO_FILL_ORDER + 1,
        )?;

        Ok((
            order,
            clearer,
            fill_bot,
            redeemed_stablecoin,
            redeemed_collateral,
        ))
    }

    pub fn simulate_redemption_results(
        redemption_amount: u64,
        prices: &TokenPrices,
        mut borrowers: Vec<UserMetadata>,
        base_rate: u16,
    ) -> RedemptionResults {
        // Log(n) algorithm that cannot really be done on chain, but illustrates
        // what should really happen.

        // this is basically the thing we can't do on chain: sort all the users at once
        // writing it here for simplicity and to calculate the correctness of the other
        // algorithm
        borrowers.sort_by(|left, right| -> cmp::Ordering {
            let mv_left = CollateralInfo::from(&left, &prices);
            let mv_right = CollateralInfo::from(&right, &prices);

            mv_left
                .collateral_ratio
                .partial_cmp(&mv_right.collateral_ratio)
                .unwrap()
        });

        borrowers.drain(..).fold(
            RedemptionResults {
                remaining_amount: redemption_amount,
                ..RedemptionResults::default()
            },
            |acc, mut borrower| -> RedemptionResults {
                let mut updated_borrowers = acc.updated_borrowers;
                if acc.remaining_amount == 0 {
                    updated_borrowers.push(borrower);
                    RedemptionResults {
                        updated_borrowers,
                        ..acc
                    }
                } else {
                    // Calculate amounts
                    let (redeemed_collateral, redeemed_stablecoin) = {
                        let redeemed_stablecoin =
                            u64::min(acc.remaining_amount, borrower.borrowed_stablecoin);

                        let redeemed_debt_dollars = redeemed_stablecoin;
                        let mv_dollars = CollateralInfo::from(&borrower, &prices).collateral_value;

                        let redeemed_col = borrower
                            .deposited_collateral
                            .to_token_map()
                            // multiply by the ratio of debt/mv
                            .mul_scalar(redeemed_debt_dollars as u128)
                            .div_scalar(mv_dollars as u128)
                            .to_collateral_amounts();

                        (redeemed_col, redeemed_stablecoin)
                    };

                    // Update the redeemed against user
                    borrower
                        .deposited_collateral
                        .sub_assign(&redeemed_collateral);

                    borrower.borrowed_stablecoin.sub_assign(redeemed_stablecoin);

                    if borrower.borrowed_stablecoin == 0 {
                        // make it inactive if fully redeemed
                        borrower
                            .inactive_collateral
                            .add_assign(&borrower.deposited_collateral);
                        borrower.deposited_collateral = CollateralAmounts::default();
                    }

                    let RedemptionCollateralSplit {
                        filler: fill_bot,
                        clearer: clear_bot,
                        stakers,
                        redeemer,
                        ..
                    } = split_redemption_collateral(&redeemed_collateral, base_rate);

                    updated_borrowers.push(borrower);

                    RedemptionResults {
                        fill_bot: acc.fill_bot.add(&fill_bot),
                        clear_bot: acc.clear_bot.add(&clear_bot),
                        stakers: acc.stakers.add(&stakers),
                        redeemer: acc.redeemer.add(&redeemer),
                        updated_borrowers,
                        redeemed_amount: acc.redeemed_amount + redeemed_stablecoin,
                        remaining_amount: acc.remaining_amount - redeemed_stablecoin,
                    }
                }
            },
        )
    }

    pub fn new_approved_user(market: &mut BorrowingMarketState) -> UserMetadata {
        let mut user = UserMetadata::default();
        let user_pubkey = Pubkey::new_unique();
        let user_metadata_pubkey = Pubkey::new_unique();

        borrowing_operations::approve_trove(market, &mut user).unwrap();

        set_addresses(&mut user, user_pubkey, user_metadata_pubkey);
        user
    }

    pub fn assert_global_collateral_unchanged(
        borrowers_before: &Vec<UserMetadata>,
        borrowers_after: &Vec<UserMetadata>,
        redeemed_collateral: &RedemptionCollateralSplit,
        clearer: &UserMetadata,
        fillers: &Vec<UserMetadata>,
        order: &RedemptionOrderInfo,
    ) {
        let total_collateral_before = borrowers_before
            .iter()
            .fold(CollateralAmounts::default(), |acc, borrower| {
                acc.add(&borrower.deposited_collateral)
            });

        let total_collateral_after =
            borrowers_after
                .iter()
                .fold(CollateralAmounts::default(), |acc, borrower| {
                    acc.add(&borrower.deposited_collateral)
                        .add(&borrower.inactive_collateral)
                });

        let redeemed_collateral_diffed = total_collateral_before.sub(&total_collateral_after);
        let redeemed_collateral_effects = CollateralAmounts::default()
            .add(&redeemed_collateral.stakers)
            .add(&redeemed_collateral.redeemer)
            .add(&redeemed_collateral.filler)
            .add(&redeemed_collateral.clearer);
        let redeemed_collateral_manual = CollateralAmounts::default()
            .add(&clearer.inactive_collateral)
            .add(&fillers[0].inactive_collateral)
            .add(&order.redeemer.inactive_collateral)
            .add(&redeemed_collateral.stakers); // unfortunately we don't side effect this one yet

        println!("Redeemed diff'ed {:?}", redeemed_collateral_diffed);
        println!("Redeemed effects {:?}", redeemed_collateral_effects);
        println!("Redeemed manual {:?}", redeemed_collateral_manual);

        assert_eq!(redeemed_collateral_diffed, redeemed_collateral_effects);
        assert_eq!(redeemed_collateral_diffed, redeemed_collateral_manual);
    }

    pub fn assert_order_status(
        redemptions_queue: RefCell<RedemptionsQueue>,
        idx: usize,
        mode: RedemptionOrderStatus,
    ) {
        assert_eq!(redemptions_queue.borrow().orders[idx].status, mode as u8);
    }
    pub fn assert_order_cleared(redemptions_queue: RefCell<RedemptionsQueue>, idx: usize) {
        assert_order_status(redemptions_queue, idx, RedemptionOrderStatus::Inactive);
    }
    pub fn assert_order_open(redemptions_queue: RefCell<RedemptionsQueue>, idx: usize) {
        assert_order_status(redemptions_queue, idx, RedemptionOrderStatus::Open);
    }

    pub fn assert_simulation_results_match(
        requested_amount: u64,
        base_rate: u16,
        prices: &TokenPrices,
        _fill_bot: &UserMetadata,
        _clear_bot: &UserMetadata,
        _redeemer: &UserMetadata,
        borrowers_before: &[UserMetadata],
        borrowers_after: &[UserMetadata],
    ) {
        let RedemptionResults {
            fill_bot,
            clear_bot,
            stakers: _,
            redeemer,
            updated_borrowers,
            redeemed_amount,
            remaining_amount: _,
        } = simulate_redemption_results(
            requested_amount,
            prices,
            borrowers_before.to_vec(),
            base_rate,
        );

        for (i, user) in updated_borrowers.iter().enumerate() {
            println!("Simulated B {} - {:?}", i, user.to_state_string());
        }

        borrowers_after
            .iter()
            .zip(updated_borrowers.iter())
            .for_each(|(actual, expected)| {
                assert_eq!(actual.borrowed_stablecoin, expected.borrowed_stablecoin);
                assert_eq!(actual.deposited_collateral, expected.deposited_collateral);
            });

        assert_eq!(requested_amount, redeemed_amount);
        assert_eq!(fill_bot, _fill_bot.inactive_collateral);
        assert_eq!(clear_bot, _clear_bot.inactive_collateral);
        assert_eq!(redeemer, _redeemer.inactive_collateral);
    }

    pub fn assert_net_value_unchanged(
        borrow_per_user: f64,
        prices: &TokenPrices,
        borrowers_before: &[UserMetadata],
        borrowers_after: &[UserMetadata],
    ) {
        let collateral_infos_before: Vec<CollateralInfo> = borrowers_before
            .iter()
            .map(|borrower| CollateralInfo::from(borrower, &prices))
            .collect();

        let collateral_infos_after: Vec<CollateralInfo> = borrowers_after
            .iter()
            .map(|borrower| {
                CollateralInfo::calculate_collateral_value(
                    borrower.borrowed_stablecoin,
                    &borrower
                        .deposited_collateral
                        .add(&borrower.inactive_collateral),
                    &prices,
                )
            })
            .collect();

        for (i, ((borrower, ci_bef), ci_aft)) in borrowers_after
            .iter()
            .zip(collateral_infos_before.iter())
            .zip(collateral_infos_after.iter())
            .enumerate()
        {
            let borrow_split = BorrowSplit::from_amount(USDH::from(borrow_per_user), 0);
            // let redeem_ratio = USDH::from(borrow_per_user * 1.005) as f64 / ci_bef.collateral_value as f64;
            let collateral_lost =
                ci_bef.collateral_value * borrow_split.amount_to_borrow / ci_bef.collateral_value; // * redeem_ratio;
            let calced_new_collateral = ci_bef.collateral_value - collateral_lost;
            println!(
                "Borrower after {} - {} - NV bef {} NV aft {} prev coll {} new coll {} calc coll {}",
                i,
                borrower.to_state_string(),
                ci_bef.net_value,
                ci_aft.net_value,
                ci_bef.collateral_value,
                ci_aft.collateral_value,
                calced_new_collateral
            );

            // for those fully redeemed
            if i < 5 {
                assert_eq!(borrower.borrowed_stablecoin, 0);
                assert_eq!(ci_aft.collateral_value, calced_new_collateral as u64);
            }

            assert_eq!(ci_bef.net_value, ci_aft.net_value);
            assert!(ci_bef.collateral_ratio <= ci_aft.collateral_ratio);
        }
    }

    pub fn assert_queue_is_empty(redemptions_queue: RefCell<RedemptionsQueue>) {
        // ensure queue is empty
        for order in redemptions_queue.borrow().orders.iter() {
            assert_eq!(order.status, RedemptionOrderStatus::Inactive as u8);
        }
    }

    pub fn assert_pending_active_users(
        redemptions_queue: RefCell<RedemptionsQueue>,
        ix: usize,
        num: usize,
    ) {
        let pending_candidate_users = redemptions_queue.borrow().orders[ix]
            .candidate_users
            .iter()
            .filter(|u| u.status == RedemptionCandidateStatus::Active as u8)
            .count();
        assert_eq!(pending_candidate_users, num);
    }

    pub fn assert_debt_burned(
        expected_redeemed_amount: u64,
        redeemed_stablecoin_effect: u64,
        borrowers_before: &Vec<UserMetadata>,
        borrowers_after: &Vec<UserMetadata>,
    ) {
        let debt_before: u64 = borrowers_before
            .iter()
            .map(|borrower| borrower.borrowed_stablecoin)
            .sum();

        let debt_after: u64 = borrowers_after
            .iter()
            .map(|borrower| borrower.borrowed_stablecoin)
            .sum();

        assert_eq!(expected_redeemed_amount, redeemed_stablecoin_effect);
        assert_eq!(expected_redeemed_amount, debt_before - debt_after);
    }

    pub fn print_candidate_users(redemptions_queue: RefCell<RedemptionsQueue>, order_id: usize) {
        for (i, candidate) in redemptions_queue.borrow().orders[order_id]
            .candidate_users
            .iter()
            .enumerate()
        {
            println!("{} - {:?}", i, candidate);
        }
    }

    pub fn print_borrowers(prefix: &str, borrowers: &Vec<UserMetadata>) {
        for (i, u) in borrowers.iter().enumerate() {
            println!("Borrowers {} {} - {}", prefix, i, u.to_state_string());
        }
    }

    pub fn print_order(prefix: &str, redemptions_queue: RefCell<RedemptionsQueue>, idx: usize) {
        let order = redemptions_queue.borrow().orders[idx];
        println!("{} {}", prefix, order.to_state_string());
    }

    pub fn assert_num_active_candidates(
        redemptions_queue: RefCell<RedemptionsQueue>,
        idx: usize,
        num: usize,
    ) {
        let all_candidates = redemptions_queue.borrow().orders[idx].candidate_users;
        let active_candidates: Vec<(u64, Pubkey)> = all_candidates
            .iter()
            .filter(|candidate| candidate.status != 0)
            .map(|candidate| (candidate.user_id, candidate.filler_metadata))
            .collect();

        assert_eq!(
            active_candidates.len(),
            num,
            "Expected len to be {} but was {}",
            num,
            active_candidates.len()
        );
    }
}
