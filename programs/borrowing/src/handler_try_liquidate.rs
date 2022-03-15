use crate::{
    borrowing_market::{
        borrowing_operations::{self},
        types::LiquidationEffects,
    },
    key, pda, stablecoin,
    state::epoch_to_scale_to_sum::{EpochToScaleToSum, LoadingMode},
    utils::oracle::get_prices,
};
use anchor_lang::prelude::*;

pub fn process(ctx: Context<crate::TryLiquidate>) -> ProgramResult {
    msg!("ix=TryLiquidate");
    // Due to lack of space in the accounts inputs,
    // we cannot transfer all the collateral at once, i.e. from sol, eth, btc
    // coll vaults to the liquidation reward vaults & liquidator ata.

    // So, instead, we just update the state, and put a LiquidationEvent
    // in a queue to be processed in a subsequent transaction. Bots will be
    // incentivised to do so, and block any other action to be done until
    // that event is processed and there are no more events to be processed.
    // As such, harvesting events is blocked, and also withdrawing and depositing,
    // because they involve harvesting.

    // There is no real problem with that, the state is correctly maintained
    // just that the liquidation reward vaults will not contain as many tokens
    // as it says in the state structs.

    // To ensure there is no problem with that, withdrawing from
    // the rewards vaults can only happen when there are no pending liquidations
    // to be transferred into the rewards vaults.

    // That check is done in the harvest_liquidation_gains instruction

    // Attempt to liquidate a user
    //  0. Check if can liquidate
    //  1. All of user's collateral is moved to stability_pool_sol_collateral_reward_vault
    //  2. State: The debt is wiped out & collateral is reduced to 0
    //  3. That much USDH is burned from the stability pool
    //  4. Liquidation gain is distributed among all stability providers
    //  5. The liquidator is paid a small fee

    let stability_pool_state = &mut ctx.accounts.stability_pool_state;
    let liquidations_queue = &mut ctx.accounts.liquidations_queue;

    let mut epoch_to_scale_to_sum =
        EpochToScaleToSum::unpack_from_zero_copy_account(&ctx.accounts.epoch_to_scale_to_sum)?;

    let prices = get_prices(
        &ctx.accounts.pyth_sol_price_info,
        &ctx.accounts.pyth_eth_price_info,
        &ctx.accounts.pyth_btc_price_info,
        &ctx.accounts.pyth_srm_price_info,
        &ctx.accounts.pyth_ray_price_info,
        &ctx.accounts.pyth_ftt_price_info,
    )?;

    let LiquidationEffects {
        liquidation_event,
        usd_to_burn_from_stability_pool,
    } = borrowing_operations::try_liquidate(
        key!(ctx, liquidator),
        &mut ctx.accounts.borrowing_market_state,
        &mut ctx.accounts.user_metadata,
        stability_pool_state,
        &mut epoch_to_scale_to_sum,
        &prices,
        &mut liquidations_queue.load_mut()?,
        ctx.accounts.clock.unix_timestamp as u64,
    )?;

    stablecoin::burn(
        usd_to_burn_from_stability_pool,
        &ctx.accounts.stablecoin_stability_pool_vault,
        &ctx.accounts.stablecoin_mint,
        &ctx.accounts.stablecoin_stability_pool_vault_authority,
        ctx.accounts
            .stability_vaults
            .stablecoin_stability_pool_vault_seed,
        pda::PDA::StabilityPool {
            owner: ctx.accounts.borrowing_market_state.initial_market_owner,
        },
        ctx.program_id,
        &ctx.accounts.token_program,
    )?;

    epoch_to_scale_to_sum
        .pack_to_zero_copy_account(&mut ctx.accounts.epoch_to_scale_to_sum, LoadingMode::Mut)?;

    msg!(
        "Liquidation successful, liquidation event {:?}",
        liquidation_event
    );

    Ok(())
}
