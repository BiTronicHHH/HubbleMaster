use anchor_lang::prelude::*;

use crate::{
    borrowing_market::{borrowing_operations, types::DepositCollateralEffects},
    token_operations::{soltoken, spltoken},
    CollateralToken,
};

pub fn process(
    ctx: Context<crate::DepositCollateral>,
    amount_in_lamports: u64,
    collateral: CollateralToken,
) -> ProgramResult {
    msg!("Depositing {:?}", collateral);
    utils::assert_permissions(&ctx, collateral)?;

    let DepositCollateralEffects {
        collateral_to_transfer_from_user,
    } = borrowing_operations::deposit_collateral(
        &mut ctx.accounts.borrowing_market_state,
        &mut ctx.accounts.user_metadata,
        amount_in_lamports,
        collateral,
    )?;

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
        ctx: &Context<crate::DepositCollateral>,
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
