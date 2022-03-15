use crate::stability_pool::stability_pool_operations;
use crate::stability_pool::types::ProvideStabilityEffects;
use crate::state::epoch_to_scale_to_sum::{EpochToScaleToSum, LoadingMode};
use crate::token_operations::stablecoin;
use anchor_lang::prelude::*;

pub fn process(ctx: Context<crate::ProvideStability>, amount: u64) -> ProgramResult {
    msg!("Providing Stability!");

    // 1. Harvest
    // 2. Send USD to the stability pool
    // 3. Update deposit and snapshot

    utils::assert_permissions(&ctx, amount)?;

    let mut epoch_to_scale_to_sum =
        EpochToScaleToSum::unpack_from_zero_copy_account(&ctx.accounts.epoch_to_scale_to_sum)?;

    // Update state
    let ProvideStabilityEffects {
        usd_to_stability_pool_transfer,
    } = stability_pool_operations::provide_stability(
        &mut ctx.accounts.stability_pool_state,
        &mut ctx.accounts.stability_provider_state,
        &mut epoch_to_scale_to_sum,
        amount,
        ctx.accounts.clock.unix_timestamp as u64,
    )?;

    // Run token transfers
    stablecoin::transfer(
        usd_to_stability_pool_transfer,
        &ctx.accounts.stablecoin_ata,
        &ctx.accounts.stablecoin_stability_pool_vault,
        &ctx.accounts.owner,
        &ctx.accounts.token_program,
    )?;

    epoch_to_scale_to_sum
        .pack_to_zero_copy_account(&mut ctx.accounts.epoch_to_scale_to_sum, LoadingMode::Mut)?;

    Ok(())
}

mod utils {
    use anchor_lang::{
        prelude::{msg, ProgramResult},
        Context,
    };
    use vipers::assert_ata;

    use crate::BorrowError;

    pub fn assert_permissions(
        ctx: &Context<crate::ProvideStability>,
        amount: u64,
    ) -> ProgramResult {
        assert_amount_not_zero(amount)?;

        assert_ata!(
            ctx.accounts.stablecoin_ata,
            ctx.accounts.owner,
            ctx.accounts.borrowing_market_state.stablecoin_mint
        );

        Ok(())
    }

    pub fn assert_amount_not_zero(amount: u64) -> ProgramResult {
        if amount == 0 {
            Err(BorrowError::StakingZero.into())
        } else {
            Ok(())
        }
    }
}
