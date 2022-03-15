use anchor_lang::prelude::*;

use crate::staking_pool::staking_pool_operations;

pub fn process(mut ctx: Context<crate::ApproveStakingPool>) -> ProgramResult {
    msg!("ix=ApproveStakingPool");

    staking_pool_operations::approve_new_user(
        &mut ctx.accounts.staking_pool_state,
        &mut ctx.accounts.user_staking_state,
    )?;

    // Set account addresses
    utils::set_accounts(&mut ctx);

    Ok(())
}

mod utils {
    use anchor_lang::Context;
    use anchor_lang::ToAccountInfo;

    use crate::key;

    pub fn set_accounts(ctx: &mut Context<crate::ApproveStakingPool>) {
        let user_staking_state = &mut ctx.accounts.user_staking_state;

        user_staking_state.owner = key!(ctx, owner);
        user_staking_state.staking_pool_state = key!(ctx, staking_pool_state);
    }
}
