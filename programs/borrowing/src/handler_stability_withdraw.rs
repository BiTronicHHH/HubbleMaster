use crate::{
    stability_pool::{stability_pool_operations, types::WithdrawStabilityEffects},
    state::epoch_to_scale_to_sum::{EpochToScaleToSum, LoadingMode},
    token_operations::stablecoin,
};
use anchor_lang::prelude::*;

pub fn process(ctx: Context<crate::WithdrawStability>, amount: u64) -> ProgramResult {
    msg!("ix=WithdrawStability");

    utils::assert_permissions(&ctx, amount)?;

    let mut epoch_to_scale_to_sum =
        EpochToScaleToSum::unpack_from_zero_copy_account(&ctx.accounts.epoch_to_scale_to_sum)?;

    let WithdrawStabilityEffects {
        usd_remaining_to_withdraw,
    } = stability_pool_operations::withdraw_stability(
        &mut ctx.accounts.stability_pool_state,
        &mut ctx.accounts.stability_provider_state,
        &mut epoch_to_scale_to_sum,
        amount,
        ctx.accounts.clock.unix_timestamp as u64,
    )?;

    stablecoin::transfer_from_stability_pool(
        usd_remaining_to_withdraw,
        ctx.accounts.borrowing_market_state.initial_market_owner,
        &ctx.accounts.stablecoin_ata,
        &ctx.accounts.stablecoin_stability_pool_vault,
        &ctx.accounts.stablecoin_stability_pool_vault_authority,
        ctx.accounts
            .stability_vaults
            .stablecoin_stability_pool_vault_seed,
        &ctx.accounts.token_program,
        ctx.program_id,
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
        ctx: &Context<crate::WithdrawStability>,
        amount: u64,
    ) -> ProgramResult {
        assert_amount_not_zero(amount)?;
        assert_has_stake(ctx.accounts.stability_provider_state.deposited_stablecoin)?;

        assert_ata!(
            ctx.accounts.stablecoin_ata,
            ctx.accounts.stability_provider_state.owner,
            ctx.accounts.borrowing_market_state.stablecoin_mint
        );

        Ok(())
    }

    fn assert_has_stake(user_total_stablecoin_provided: u64) -> ProgramResult {
        if user_total_stablecoin_provided == 0 {
            Err(BorrowError::NothingToUnstake.into())
        } else {
            Ok(())
        }
    }

    pub fn assert_amount_not_zero(amount: u64) -> ProgramResult {
        if amount == 0 {
            Err(BorrowError::NothingToUnstake.into())
        } else {
            Ok(())
        }
    }
}
