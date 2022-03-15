use anchor_lang::prelude::*;

use crate::borrowing_market::borrowing_operations;

pub fn process(mut ctx: Context<crate::ApproveTrove>) -> ProgramResult {
    msg!("ix=ApproveTrove");
    utils::assert_permissions(&ctx)?;

    borrowing_operations::approve_trove(
        &mut ctx.accounts.borrowing_market_state,
        &mut ctx.accounts.user_metadata,
    )?;

    // Set account addresses
    utils::set_accounts(&mut ctx);

    Ok(())
}

mod utils {
    use anchor_lang::prelude::{msg, ProgramResult};
    use anchor_lang::Context;
    use anchor_lang::ToAccountInfo;
    use vipers::assert_ata;

    use crate::borrowing_market::borrowing_operations::utils::set_addresses;
    use crate::key;

    pub fn assert_permissions(ctx: &Context<crate::ApproveTrove>) -> ProgramResult {
        assert_ata!(
            ctx.accounts.stablecoin_ata,
            ctx.accounts.owner,
            ctx.accounts.borrowing_market_state.stablecoin_mint,
        );

        Ok(())
    }

    pub fn set_accounts(ctx: &mut Context<crate::ApproveTrove>) {
        let mut user_metadata = &mut ctx.accounts.user_metadata;

        user_metadata.stablecoin_ata = key!(ctx, stablecoin_ata);
        user_metadata.borrowing_market_state = key!(ctx, borrowing_market_state);

        let metadata_pk = key!(user_metadata);
        let owner_pk = key!(ctx, owner);

        set_addresses(&mut user_metadata, owner_pk, metadata_pk);
    }
}
