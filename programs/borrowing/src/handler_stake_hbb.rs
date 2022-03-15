use crate::staking_pool::staking_pool_operations;
use crate::token_operations::hbb;

use anchor_lang::{prelude::ProgramResult, Context};

pub fn process(ctx: Context<crate::StakeHbbStakingPool>, amount: u64) -> ProgramResult {
    utils::assert_permissions(&ctx, amount)?;

    staking_pool_operations::user_stake(
        &mut ctx.accounts.staking_pool_state,
        &mut ctx.accounts.user_staking_state,
        amount,
    );

    hbb::transfer(
        amount,
        &ctx.accounts.user_hbb_staking_ata,
        &ctx.accounts.staking_vault,
        &ctx.accounts.owner,
        &ctx.accounts.token_program,
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
        ctx: &Context<crate::StakeHbbStakingPool>,
        amount: u64,
    ) -> ProgramResult {
        assert_amount_not_zero(amount)?;

        assert_ata!(
            ctx.accounts.user_hbb_staking_ata,
            ctx.accounts.owner,
            ctx.accounts.borrowing_market_state.hbb_mint,
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
