use anchor_lang::prelude::*;
use anchor_spl::dex;

/// Initializes an open orders account (only used once before interacting with a market)
///
/// # Arguments
///
/// * `open_orders` - A new account initialized by the order_payer_authority,
///   before the call to this function
/// * `authority` - The authority that is going to sign the new order instruction,
///   where we send the coins we want to swap to the serum market
/// * `market` - The address of the serum market (WSOL/BTC/ETH - USDC)
pub fn process(ctx: Context<crate::SerumInitOpenOrders>) -> ProgramResult {
    msg!("Ix=SerumInitOpenOrders");

    let dex_accs = dex::InitOpenOrders {
        open_orders: ctx.accounts.open_orders.clone(),
        authority: ctx.accounts.order_payer_authority.clone(),
        market: ctx.accounts.market.clone(),
        rent: ctx.accounts.rent.to_account_info().clone(),
    };

    let ctx = CpiContext::new(ctx.accounts.dex_program.clone(), dex_accs);
    dex::init_open_orders(ctx)?;

    Ok(())
}
