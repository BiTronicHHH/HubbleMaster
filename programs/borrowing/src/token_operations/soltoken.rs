use anchor_lang::prelude::{msg, AccountInfo, ProgramResult};
use spl_token::error::TokenError;
pub fn transfer_from_user<'info>(
    amount_in_lamports: u64,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) -> ProgramResult {
    let ix = anchor_lang::solana_program::system_instruction::transfer(
        from.key,
        to.key,
        amount_in_lamports,
    );

    anchor_lang::solana_program::program::invoke(
        &ix,
        &[from.clone(), to.clone(), system_program.clone()],
    )
}

pub fn transfer_from_vault<'info>(
    amount_in_lamports: u64,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
) -> ProgramResult {
    if amount_in_lamports == 0 {
        return Ok(());
    }

    let source_starting_lamports = from.lamports();
    let dest_starting_lamports = to.lamports();

    msg!(
        "Transferring lamports {} from {:?} with amount {}",
        amount_in_lamports,
        from.key,
        source_starting_lamports
    );

    msg!(
        "Transferring lamports {} to {:?} with amount {}",
        amount_in_lamports,
        to.key,
        dest_starting_lamports
    );

    **from.lamports.borrow_mut() = source_starting_lamports
        .checked_sub(amount_in_lamports)
        .ok_or(TokenError::Overflow)?;

    **to.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(amount_in_lamports)
        .ok_or(TokenError::Overflow)?;

    Ok(())
}
