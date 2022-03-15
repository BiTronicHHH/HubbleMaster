use anchor_lang::prelude::*;
pub use anchor_lang::solana_program::native_token::{lamports_to_sol, sol_to_lamports};

use crate::{
    borrowing_market::{borrowing_operations, types::DepositAndBorrowEffects},
    stablecoin,
    state::CollateralToken,
    token_operations::{soltoken, spltoken},
    utils::oracle::get_prices,
};

pub fn process(
    ctx: Context<crate::DepositCollateralAndBorrowStable>,
    deposit_amount: u64,
    collateral: CollateralToken,
    borrow_amount: u64,
) -> ProgramResult {
    msg!("Depositing and borrowing {:?} ", collateral);
    utils::assert_permissions(&ctx, collateral)?;

    let prices = get_prices(
        &ctx.accounts.pyth_sol_price_info,
        &ctx.accounts.pyth_eth_price_info,
        &ctx.accounts.pyth_btc_price_info,
        &ctx.accounts.pyth_srm_price_info,
        &ctx.accounts.pyth_ray_price_info,
        &ctx.accounts.pyth_ftt_price_info,
    )?;

    let borrowing_market_state = &mut ctx.accounts.borrowing_market_state;

    let DepositAndBorrowEffects {
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault,
        collateral_to_transfer_from_user,
    } = borrowing_operations::deposit_and_borrow(
        borrowing_market_state,
        &mut ctx.accounts.user_metadata,
        &mut ctx.accounts.staking_pool_state,
        borrow_amount,
        deposit_amount,
        collateral,
        &prices,
        ctx.accounts.clock.unix_timestamp as u64,
    )?;

    // Mint USDH to user
    stablecoin::mint(
        amount_mint_to_user,
        borrowing_market_state.stablecoin_mint_seed,
        borrowing_market_state.initial_market_owner,
        ctx.program_id,
        ctx.accounts.stablecoin_mint.clone(),
        ctx.accounts.stablecoin_borrowing_associated_account.clone(),
        ctx.accounts.stablecoin_mint_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
    )?;

    // Mint USDH to the stakers vault (borrowing fees vault)
    stablecoin::mint(
        amount_mint_to_fees_vault,
        borrowing_market_state.stablecoin_mint_seed,
        borrowing_market_state.initial_market_owner,
        ctx.program_id,
        ctx.accounts.stablecoin_mint.clone(),
        ctx.accounts.borrowing_fees_vault.clone(),
        ctx.accounts.stablecoin_mint_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
    )?;

    // Mint USDH to the treasury vault
    stablecoin::mint(
        amount_mint_to_treasury_vault,
        borrowing_market_state.stablecoin_mint_seed,
        borrowing_market_state.initial_market_owner,
        ctx.program_id,
        ctx.accounts.stablecoin_mint.clone(),
        ctx.accounts.treasury_vault.clone(),
        ctx.accounts.stablecoin_mint_authority.clone(),
        ctx.accounts.token_program.to_account_info(),
    )?;

    // Deposit collateral from user's ATA to vault
    let amount = collateral_to_transfer_from_user.token_amount(collateral) as u64;
    match collateral {
        CollateralToken::SOL => soltoken::transfer_from_user(
            amount,
            &ctx.accounts.collateral_from,
            &ctx.accounts.collateral_to,
            &ctx.accounts.system_program,
        ),
        _ => spltoken::transfer_from_user(
            amount,
            &ctx.accounts.collateral_from,
            &ctx.accounts.collateral_to,
            &ctx.accounts.owner,
            &ctx.accounts.token_program,
        ),
    }?;

    msg!(
        "Deposited {} of {}, Borrowed {} USDH + stakers fee {} + treasury fee {}",
        deposit_amount,
        collateral as u8,
        amount_mint_to_user,
        amount_mint_to_fees_vault,
        amount_mint_to_treasury_vault
    );

    Ok(())
}

mod utils {
    use crate::CollateralToken;
    use anchor_lang::{
        prelude::{msg, ProgramResult},
        Context, Key,
    };
    use vipers::{assert_ata, assert_keys_eq};

    pub fn assert_permissions(
        ctx: &Context<crate::DepositCollateralAndBorrowStable>,
        collateral: CollateralToken,
    ) -> ProgramResult {
        let borrowing_vaults = &ctx.accounts.borrowing_vaults;
        let collateral_to = ctx.accounts.collateral_to.key;

        assert_keys_eq!(
            borrowing_vaults.vault_address(collateral),
            collateral_to,
            "To vault does not match borrowing market collateral vault"
        );

        if collateral != CollateralToken::SOL {
            assert_ata!(
                ctx.accounts.collateral_from,
                ctx.accounts.user_metadata.owner,
                borrowing_vaults.mint_address(collateral),
            );
        } else {
            assert_keys_eq!(
                ctx.accounts.owner.key,
                ctx.accounts.collateral_from.key,
                "From account should be the owner native account"
            );
        }

        Ok(())
    }
}
