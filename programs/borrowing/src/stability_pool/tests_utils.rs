#![allow(unaligned_references)]
#[cfg(test)]
pub mod utils {

    use anchor_lang::solana_program::native_token::sol_to_lamports;

    use crate::{
        assert_fuzzy_eq, stability_pool::stability_pool_operations,
        state::epoch_to_scale_to_sum::EpochToScaleToSum, utils::coretypes::USDH, CollateralAmounts,
        StabilityPoolState, StabilityProviderState,
    };
    // TODO: fix this and every single assert_fuzzy_equal
    const SE: u64 = 512;

    pub fn new_stability_users(
        stability_pool_state: &mut StabilityPoolState,
        epoch_to_scale_to_sum: &mut EpochToScaleToSum,
        count: usize,
        stability_to_provide: f64,
    ) -> Vec<StabilityProviderState> {
        (0..count)
            .map(|_| {
                let mut user = StabilityProviderState::default();

                stability_pool_operations::approve_new_user(stability_pool_state, &mut user);

                stability_pool_operations::provide_stability(
                    stability_pool_state,
                    &mut user,
                    epoch_to_scale_to_sum,
                    USDH::from(stability_to_provide),
                    0,
                )
                .unwrap();

                user
            })
            .collect()
    }

    pub fn assert_balances(
        users: &[StabilityProviderState],
        stability_pool_state: &StabilityPoolState,
        expected_total_usd_deposits: f64,
        expected_user_collateral_gained: Vec<f64>,
        expected_user_stability_provided: Vec<f64>,
        expected_pending_gains: f64,
        expected_total_gains: f64,
        expected_user_collateral_gained_epsilon: Option<u64>,
    ) {
        // expected_total_usd_deposits
        let actual_total_user_deposits = &stability_pool_state.stablecoin_deposited;
        assert_fuzzy_eq!(
            *actual_total_user_deposits,
            USDH::from(expected_total_usd_deposits),
            SE
        );

        // expected_user_collateral_gained
        for (i, user) in users.iter().enumerate() {
            let user_gains_cumulative = &user.cumulative_gains_per_user;
            assert_fuzzy_eq!(
                user_gains_cumulative.sol,
                sol_to_lamports(expected_user_collateral_gained[i]),
                expected_user_collateral_gained_epsilon.unwrap_or(SE)
            );
        }

        // expected_user_stability_provided
        for (i, user) in users.iter().enumerate() {
            let actual_user_deposits = &user.deposited_stablecoin;
            assert_fuzzy_eq!(
                *actual_user_deposits,
                USDH::from(expected_user_stability_provided[i]) as u128,
                SE
            );
        }

        let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
        let total_gains_pending = &stability_pool_state.pending_collateral_gains;

        assert_eq!(
            total_gains_pending.sol,
            sol_to_lamports(expected_pending_gains) as u128
        );

        assert_fuzzy_eq!(
            total_gains_cumulative.sol,
            sol_to_lamports(expected_total_gains),
            SE
        );
    }

    #[rustfmt::skip]
    pub fn assert_balances_multicollateral(
        users: Vec<&StabilityProviderState>,
        stability_pool_state: &StabilityPoolState,
        expected_total_usd_deposits: f64,
        expected_user_collateral_gained: Vec<CollateralAmounts>,
        expected_user_stability_provided: Vec<f64>,
        expected_pending_gains: CollateralAmounts,
        expected_total_gains: CollateralAmounts,
        epsilon: Option<u64>,
    ) {
        // expected_total_usd_deposits
        let actual_total_user_deposits = &stability_pool_state.stablecoin_deposited;
        assert_fuzzy_eq!(
            *actual_total_user_deposits,
            USDH::from(expected_total_usd_deposits),
            SE
        );

        // expected_user_collateral_gained
        for (i, user) in users.iter().enumerate() {
            let user_gains_cumulative = &user.cumulative_gains_per_user;
            assert_fuzzy_eq!(user_gains_cumulative.sol, expected_user_collateral_gained[i].sol, epsilon.unwrap_or(SE));
            assert_fuzzy_eq!(user_gains_cumulative.eth, expected_user_collateral_gained[i].eth, epsilon.unwrap_or(SE));
            assert_fuzzy_eq!(user_gains_cumulative.btc, expected_user_collateral_gained[i].btc, epsilon.unwrap_or(SE));
            assert_fuzzy_eq!(user_gains_cumulative.srm, expected_user_collateral_gained[i].srm, epsilon.unwrap_or(SE));
            assert_fuzzy_eq!(user_gains_cumulative.ray, expected_user_collateral_gained[i].ray, epsilon.unwrap_or(SE));
            assert_fuzzy_eq!(user_gains_cumulative.ftt, expected_user_collateral_gained[i].ftt, epsilon.unwrap_or(SE));
        }

        // expected_user_stability_provided
        for (i, user) in users.iter().enumerate() {
            let actual_user_deposits = &user.deposited_stablecoin;
            assert_fuzzy_eq!(
                *actual_user_deposits,
                USDH::from(expected_user_stability_provided[i]) as u128,
                SE
            );
        }

        let total_gains_cumulative = &stability_pool_state.cumulative_gains_total;
        let total_gains_pending = &stability_pool_state.pending_collateral_gains;

        assert_fuzzy_eq!(total_gains_pending.sol, expected_pending_gains.sol as u128, epsilon.unwrap_or(SE));
        assert_fuzzy_eq!(total_gains_pending.eth, expected_pending_gains.eth as u128, epsilon.unwrap_or(SE));
        assert_fuzzy_eq!(total_gains_pending.btc, expected_pending_gains.btc as u128, epsilon.unwrap_or(SE));
        assert_fuzzy_eq!(total_gains_pending.srm, expected_pending_gains.srm as u128, epsilon.unwrap_or(SE));
        assert_fuzzy_eq!(total_gains_pending.ray, expected_pending_gains.ray as u128, epsilon.unwrap_or(SE));
        assert_fuzzy_eq!(total_gains_pending.ftt, expected_pending_gains.ftt as u128, epsilon.unwrap_or(SE));

        assert_fuzzy_eq!(total_gains_cumulative.sol, expected_total_gains.sol, SE);
        assert_fuzzy_eq!(total_gains_cumulative.eth, expected_total_gains.eth, SE);
        assert_fuzzy_eq!(total_gains_cumulative.btc, expected_total_gains.btc, SE);
        assert_fuzzy_eq!(total_gains_cumulative.srm, expected_total_gains.srm, SE);
        assert_fuzzy_eq!(total_gains_cumulative.ray, expected_total_gains.ray, SE);
        assert_fuzzy_eq!(total_gains_cumulative.ftt, expected_total_gains.ftt, SE);
    }
}
