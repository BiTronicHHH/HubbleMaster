use anchor_lang::prelude::*;

use crate::key;
use crate::stability_pool::stability_pool_operations;

pub fn process(ctx: Context<crate::ApproveProvideStability>) -> ProgramResult {
    let stability_provider_state = &mut ctx.accounts.stability_provider_state;
    let stability_pool_state = &mut ctx.accounts.stability_pool_state;

    stability_provider_state.version = 0;
    stability_provider_state.stability_pool_state = key!(stability_pool_state);
    stability_provider_state.owner = key!(ctx, owner);

    stability_pool_operations::approve_new_user(stability_pool_state, stability_provider_state);

    Ok(())
}
