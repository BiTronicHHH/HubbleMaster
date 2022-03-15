use anchor_lang::{prelude::ProgramResult, Context};

use crate::{
    borrowing_market::{borrowing_operations, types::RepayLoanEffects},
    pda, RepayLoan,
};

pub fn process(ctx: Context<RepayLoan>, stablecoin_amount: u64) -> ProgramResult {
    let borrowing_market_state = &mut ctx.accounts.borrowing_market_state;
    let borrowing_vaults = &ctx.accounts.borrowing_vaults;

    let RepayLoanEffects {
        amount_to_burn,
        amount_to_transfer,
    } = borrowing_operations::repay_loan(
        borrowing_market_state,
        &mut ctx.accounts.user_metadata,
        stablecoin_amount,
    )?;

    // 1. Transfer the amount of debt from user associated account to burning pot
    crate::stablecoin::transfer(
        amount_to_transfer,
        &ctx.accounts.stablecoin_borrowing_associated_account,
        &ctx.accounts.burning_vault,
        &ctx.accounts.owner,
        &ctx.accounts.token_program,
    )?;

    // 2. Burn from burning pot
    crate::stablecoin::burn(
        amount_to_burn,
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

    Ok(())
}
