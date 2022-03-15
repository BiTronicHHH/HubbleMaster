use anchor_lang::prelude::msg;
#[cfg(not(test))]
use anchor_lang::solana_program::log::sol_log_compute_units;
use anchor_lang::{prelude::ProgramResult, Context, ToAccountInfo};

use crate::key;
use crate::redemption::redemption_operations;
use crate::{log_compute_units, FillRedemptionOrder};

pub fn process(ctx: Context<FillRedemptionOrder>, order_id: u64) -> ProgramResult {
    log_compute_units!("ix=FillRedemptionOrder - Before Extract Candidates");
    let borrowing_market_state_pk = key!(ctx, borrowing_market_state);
    let mut metadata_accounts =
        utils::deserialize_remaining_user_metadatas(&ctx, &borrowing_market_state_pk)?;
    let mut submitted_candidates = utils::accounts_to_metadatas(&mut metadata_accounts);
    let filler_metadata = &mut ctx.accounts.filler_metadata;
    let redemptions_queue = &mut ctx.accounts.redemptions_queue.load_mut()?;
    let borrowing_market_state = &mut ctx.accounts.borrowing_market_state;
    let timestamp = ctx.accounts.clock.unix_timestamp as u64;

    msg!(
        "User {:?} filling redemption order {} with {} candidates",
        filler_metadata.metadata_pk,
        order_id,
        submitted_candidates.len(),
    );

    log_compute_units!("Fill Redemption Order - Before Fill");
    redemption_operations::fill_redemption_order(
        order_id,
        borrowing_market_state,
        redemptions_queue,
        &mut submitted_candidates,
        filler_metadata,
        timestamp,
    )?;

    log_compute_units!("Fill Redemption Order - After Merge");
    utils::serialize_user_metadatas(&ctx, &mut metadata_accounts);

    Ok(())
}

pub mod utils {
    use std::ops::DerefMut;

    use anchor_lang::__private::ErrorCode;
    use anchor_lang::prelude::Pubkey;
    use anchor_lang::{Context, ProgramAccount};

    use crate::UserMetadata;

    pub fn deserialize_remaining_user_metadatas<'a, 'info, T>(
        ctx: &'a Context<'_, '_, '_, 'info, T>,
        borrowing_market_state: &'a Pubkey,
    ) -> Result<Vec<ProgramAccount<'info, UserMetadata>>, ErrorCode> {
        let metadata_program_accounts = ctx
            .remaining_accounts
            .iter()
            .filter_map(|unsafe_acc| {
                if !unsafe_acc.is_writable {
                    None
                } else {
                    ProgramAccount::<UserMetadata>::try_from(ctx.program_id, unsafe_acc).ok()
                }
            })
            .map(|user_metadata| {
                if &user_metadata.borrowing_market_state != borrowing_market_state {
                    return Err(ErrorCode::ConstraintHasOne);
                }
                Ok(user_metadata)
            })
            .collect();
        metadata_program_accounts
    }

    pub fn accounts_to_metadatas<'a>(
        submitted_candidates_p: &'a mut Vec<ProgramAccount<UserMetadata>>,
    ) -> Vec<&'a mut UserMetadata> {
        let candidates: Vec<&mut UserMetadata> = submitted_candidates_p
            .iter_mut()
            .map(|x| x.deref_mut())
            .collect();
        candidates
    }

    pub fn serialize_user_metadatas<T>(
        ctx: &Context<T>,
        submitted_candidates_p: &mut Vec<ProgramAccount<UserMetadata>>,
    ) {
        submitted_candidates_p
            .iter_mut()
            .for_each(|acc| anchor_lang::AccountsExit::exit(acc, ctx.program_id).unwrap());
    }
}
