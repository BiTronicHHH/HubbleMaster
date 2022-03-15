use anchor_lang::prelude::msg;
use anchor_lang::solana_program::log::sol_log_compute_units;
use anchor_lang::{prelude::ProgramResult, Context, ToAccountInfo};

use crate::handler_fill_redemption_order::utils::{
    accounts_to_metadatas, deserialize_remaining_user_metadatas, serialize_user_metadatas,
};
use crate::key;
use crate::redemption::redemption_operations;
use crate::redemption::types::ClearRedemptionOrderEffects;
use crate::utils::pda;
use crate::ClearRedemptionOrder;

pub fn process(ctx: Context<ClearRedemptionOrder>, order_id: u64) -> ProgramResult {
    let borrowing_market_state_pk = key!(ctx, borrowing_market_state);
    let mut metadata_accounts =
        deserialize_remaining_user_metadatas(&ctx, &borrowing_market_state_pk)?;
    let mut fillers_and_borrowers = accounts_to_metadatas(&mut metadata_accounts);
    let clearer_metadata = &mut ctx.accounts.clearer_metadata;
    let redeemer_metadata = &mut ctx.accounts.redeemer_metadata;
    let borrowing_market_state = &mut ctx.accounts.borrowing_market_state;
    let borrowing_vaults = &ctx.accounts.borrowing_vaults;
    let redemptions_queue = &mut ctx.accounts.redemptions_queue.load_mut()?;
    let timestamp = ctx.accounts.clock.unix_timestamp as u64;

    msg!("BEFORE EXTRACT CANDIDATES OR FILLERS");
    sol_log_compute_units();

    msg!(
        "User {:?} clearing redemption order {} with {} fillers and borrowers",
        clearer_metadata.metadata_pk,
        order_id,
        fillers_and_borrowers.len(),
    );

    msg!("BEFORE CLEAR");
    sol_log_compute_units();
    let ClearRedemptionOrderEffects {
        redeemed_stablecoin,
        redeemed_collateral: _redeemed_collateral,
    } = redemption_operations::clear_redemption_order(
        order_id,
        redeemer_metadata,
        clearer_metadata,
        borrowing_market_state,
        redemptions_queue,
        &mut fillers_and_borrowers,
        timestamp,
    )?;

    msg!("BEFORE BURN");
    sol_log_compute_units();
    // Burn from burning pot
    crate::stablecoin::burn(
        redeemed_stablecoin,
        &ctx.accounts.burning_vault,
        &ctx.accounts.stablecoin_mint,
        &ctx.accounts.burning_vault_authority,
        borrowing_vaults.burning_vault_seed,
        pda::PDA::BurningPotAccount {
            owner: borrowing_market_state.initial_market_owner,
        },
        ctx.program_id,
        &ctx.accounts.token_program,
    )?;
    msg!("BEFORE WRITE");
    sol_log_compute_units();

    serialize_user_metadatas(&ctx, &mut metadata_accounts);
    Ok(())
}
