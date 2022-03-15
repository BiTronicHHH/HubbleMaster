use anchor_lang::{prelude::ProgramResult, Context};

use crate::redemption::redemption_operations;
use crate::redemption::types::AddRedemptionOrderEffects;
use crate::utils::oracle::get_prices;
use crate::AddRedemptionOrder;

pub fn process(ctx: Context<AddRedemptionOrder>, stablecoin_amount: u64) -> ProgramResult {
    let redeemer_metadata = &mut ctx.accounts.redeemer_metadata;
    let redemptions_queue = &mut ctx.accounts.redemptions_queue.load_mut()?;
    let timestamp = ctx.accounts.clock.unix_timestamp as u64;

    let prices = get_prices(
        &ctx.accounts.pyth_sol_price_info,
        &ctx.accounts.pyth_eth_price_info,
        &ctx.accounts.pyth_btc_price_info,
        &ctx.accounts.pyth_srm_price_info,
        &ctx.accounts.pyth_ray_price_info,
        &ctx.accounts.pyth_ftt_price_info,
    )?;

    let AddRedemptionOrderEffects {
        transfer_stablecoin_amount,
        ..
    } = redemption_operations::add_redemption_order(
        redeemer_metadata,
        redemptions_queue,
        &mut ctx.accounts.borrowing_market_state,
        &prices,
        timestamp,
        stablecoin_amount,
    )?;

    crate::stablecoin::transfer(
        transfer_stablecoin_amount,
        &ctx.accounts.redeemer_stablecoin_associated_account,
        &ctx.accounts.burning_vault,
        &ctx.accounts.redeemer,
        &ctx.accounts.token_program,
    )?;
    Ok(())
}
