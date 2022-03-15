use anchor_lang::prelude::*;
pub use anchor_lang::solana_program::native_token::{lamports_to_sol, sol_to_lamports};

use crate::{
    borrowing_market::{borrowing_operations, types::BorrowStablecoinEffects},
    stablecoin,
    utils::oracle::get_prices,
};

pub fn process(ctx: Context<crate::BorrowStable>, stablecoin_amount: u64) -> ProgramResult {
    let prices = get_prices(
        &ctx.accounts.pyth_sol_price_info,
        &ctx.accounts.pyth_eth_price_info,
        &ctx.accounts.pyth_btc_price_info,
        &ctx.accounts.pyth_srm_price_info,
        &ctx.accounts.pyth_ray_price_info,
        &ctx.accounts.pyth_ftt_price_info,
    )?;

    let borrowing_market_state = &mut ctx.accounts.borrowing_market_state;

    let BorrowStablecoinEffects {
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault,
    } = borrowing_operations::borrow_stablecoin(
        borrowing_market_state,
        &mut ctx.accounts.user_metadata,
        &mut ctx.accounts.staking_pool_state,
        stablecoin_amount,
        &prices,
        ctx.accounts.clock.unix_timestamp as u64,
    )?;

    // Mint USDH to user
    stablecoin::mint(
        amount_mint_to_user,
        borrowing_market_state.stablecoin_mint_seed,
        borrowing_market_state.initial_market_owner,
        ctx.program_id,
        ctx.accounts.stablecoin_mint.clone(),
        ctx.accounts.stablecoin_borrowing_associated_account.clone(),
        ctx.accounts.stablecoin_mint_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
    )?;

    // Mint USDH to the stakers vault (borrowing fees vault)
    stablecoin::mint(
        amount_mint_to_fees_vault,
        borrowing_market_state.stablecoin_mint_seed,
        borrowing_market_state.initial_market_owner,
        ctx.program_id,
        ctx.accounts.stablecoin_mint.clone(),
        ctx.accounts.borrowing_fees_vault.clone(),
        ctx.accounts.stablecoin_mint_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
    )?;

    // Mint USDH to the treasury vault
    stablecoin::mint(
        amount_mint_to_treasury_vault,
        borrowing_market_state.stablecoin_mint_seed,
        borrowing_market_state.initial_market_owner,
        ctx.program_id,
        ctx.accounts.stablecoin_mint.clone(),
        ctx.accounts.treasury_vault.clone(),
        ctx.accounts.stablecoin_mint_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
    )?;

    msg!(
        "Borrowed {} USDH + fee {}",
        amount_mint_to_user,
        amount_mint_to_fees_vault
    );

    Ok(())
}
