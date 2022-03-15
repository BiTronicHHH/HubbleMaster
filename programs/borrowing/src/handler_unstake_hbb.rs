use crate::staking_pool::{staking_pool_operations, types::UnstakeEffects};
use crate::token_operations::{hbb, stablecoin};
use anchor_lang::prelude::*;

pub fn process(ctx: Context<crate::UnstakeHbbStakingPool>, amount: u64) -> ProgramResult {
    utils::assert_permissions(&ctx, amount)?;

    let borrowing_market_state = &ctx.accounts.borrowing_market_state;
    let borrowing_vaults = &ctx.accounts.borrowing_vaults;
    let staking_pool_state = &mut ctx.accounts.staking_pool_state;
    let user_staking_state = &mut ctx.accounts.user_staking_state;

    let UnstakeEffects {
        reward,
        amount_to_withdraw,
    } = staking_pool_operations::user_unstake(staking_pool_state, user_staking_state, amount)?;

    if reward > 0 {
        stablecoin::transfer_from_borrowing_fees_vault(
            reward as u64,
            borrowing_market_state.initial_market_owner,
            &ctx.accounts.user_stablecoin_rewards_ata,
            &ctx.accounts.borrowing_fees_vault,
            &ctx.accounts.borrowing_fees_vault_authority,
            borrowing_vaults.borrowing_fees_vault_seed,
            &ctx.accounts.token_program,
            ctx.program_id,
        )?;
    }

    hbb::transfer_from_staking_pool(
        amount_to_withdraw,
        borrowing_market_state.initial_market_owner,
        &ctx.accounts.user_hbb_staking_ata,
        &ctx.accounts.staking_vault,
        &ctx.accounts.staking_vault_authority,
        staking_pool_state.staking_vault_seed,
        &ctx.accounts.token_program,
        ctx.program_id,
    )?;

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
        ctx: &Context<crate::UnstakeHbbStakingPool>,
        amount: u64,
    ) -> ProgramResult {
        assert_amount_not_zero(amount)?;

        assert_ata!(
            ctx.accounts.user_hbb_staking_ata,
            ctx.accounts.owner,
            ctx.accounts.borrowing_market_state.hbb_mint,
        );

        assert_ata!(
            ctx.accounts.user_stablecoin_rewards_ata,
            ctx.accounts.owner,
            ctx.accounts.borrowing_market_state.stablecoin_mint,
        );

        Ok(())
    }

    pub fn assert_amount_not_zero(amount: u64) -> ProgramResult {
        if amount == 0 {
            Err(BorrowError::NothingToUnstake.into())
        } else {
            Ok(())
        }
    }
}
