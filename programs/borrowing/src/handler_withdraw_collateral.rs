use anchor_lang::{prelude::*, AccountsClose};

use crate::{
    borrowing_market::{borrowing_operations, types::WithdrawCollateralEffects},
    soltoken,
    token_operations::spltoken,
    utils::{oracle::get_prices, pda::PDA},
    CollateralToken,
};

pub fn process(
    ctx: Context<crate::WithdrawCollateral>,
    amount: u64,
    collateral: CollateralToken,
) -> ProgramResult {
    msg!("Ix=WithdrawCollateral {} {:?}", amount, collateral);
    utils::assert_permissions(&ctx, collateral)?;

    let prices = get_prices(
        &ctx.accounts.pyth_sol_price_info,
        &ctx.accounts.pyth_eth_price_info,
        &ctx.accounts.pyth_btc_price_info,
        &ctx.accounts.pyth_srm_price_info,
        &ctx.accounts.pyth_ray_price_info,
        &ctx.accounts.pyth_ftt_price_info,
    )?;

    let WithdrawCollateralEffects {
        collateral_to_transfer_to_user,
        close_user_metadata,
    } = borrowing_operations::withdraw_collateral(
        &mut ctx.accounts.borrowing_market_state,
        &mut ctx.accounts.user_metadata,
        amount,
        collateral,
        &prices,
    )?;

    match collateral {
        CollateralToken::SOL => soltoken::transfer_from_vault(
            collateral_to_transfer_to_user.sol as u64,
            &ctx.accounts.collateral_from,
            &ctx.accounts.owner,
        ),
        _ => spltoken::transfer_from_vault(
            amount,
            PDA::collateral_vault_from(&ctx.accounts.borrowing_market_state.initial_market_owner),
            &ctx.accounts.collateral_to,
            &ctx.accounts.collateral_from,
            &ctx.accounts.collateral_from_authority,
            ctx.accounts.borrowing_vaults.collateral_vaults_seed,
            &ctx.accounts.token_program,
            ctx.program_id,
        ),
    }?;

    if close_user_metadata {
        ctx.accounts
            .user_metadata
            .close(ctx.accounts.owner.clone())?;
    }

    Ok(())
}

mod utils {

    use anchor_lang::{
        prelude::{msg, ProgramResult},
        Context, Key,
    };
    use vipers::{assert_ata, assert_keys_eq};

    use crate::CollateralToken;

    pub fn assert_permissions(
        ctx: &Context<crate::WithdrawCollateral>,
        collateral: CollateralToken,
    ) -> ProgramResult {
        let borrowing_vaults = &ctx.accounts.borrowing_vaults;

        let from_vault = ctx.accounts.collateral_from.key;
        let from_authority = ctx.accounts.collateral_from_authority.key;

        assert_keys_eq!(
            borrowing_vaults.vault_address(collateral),
            from_vault,
            "From vault does not match borrowing market collateral vault"
        );

        if collateral != CollateralToken::SOL {
            assert_keys_eq!(
                borrowing_vaults.collateral_vaults_authority,
                from_authority,
                "From vault authority does not match borrowing market collateral vaults authority"
            );
            assert_ata!(
                ctx.accounts.collateral_to,
                ctx.accounts.user_metadata.owner,
                borrowing_vaults.mint_address(collateral)
            );
        } else {
            assert_keys_eq!(
                ctx.accounts.owner.key,
                ctx.accounts.collateral_to.key,
                "To account should be the owner native account"
            );
        }

        Ok(())
    }
}
