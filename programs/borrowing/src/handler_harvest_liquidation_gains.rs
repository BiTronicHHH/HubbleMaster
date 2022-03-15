use anchor_lang::prelude::*;

use crate::{
    stability_pool::{stability_pool_operations, types::HarvestLiquidationGainsEffects},
    state::epoch_to_scale_to_sum::{EpochToScaleToSum, LoadingMode},
    token_operations::{self, spltoken},
    utils::pda::PDA,
    StabilityToken::{self},
};

pub fn process(
    ctx: Context<crate::HarvestLiquidationGains>,
    harvest_token: StabilityToken,
) -> ProgramResult {
    msg!("Ix=HarvestLiquidationGains {:?}", harvest_token);
    utils::assert_permissions(&ctx, harvest_token)?;
    let mut epoch_to_scale_to_sum =
        EpochToScaleToSum::unpack_from_zero_copy_account(&ctx.accounts.epoch_to_scale_to_sum)?;

    // Calculate pending gains
    let HarvestLiquidationGainsEffects { gains } =
        stability_pool_operations::harvest_liquidation_gains(
            &mut ctx.accounts.stability_pool_state,
            &mut ctx.accounts.stability_provider_state,
            &mut epoch_to_scale_to_sum,
            &mut ctx.accounts.liquidations_queue.load_mut()?,
            ctx.accounts.clock.unix_timestamp as u64,
            harvest_token,
        )?;

    match harvest_token {
        StabilityToken::HBB => {
            // do nothing - we always mint any HBB gains
        }
        StabilityToken::SOL => {
            // Transfer SOL
            let amount = gains.sol as u64;
            if amount > 0 {
                token_operations::soltoken::transfer_from_vault(
                    amount,
                    &ctx.accounts.liquidation_rewards_vault,
                    &ctx.accounts.owner,
                )?;
            }
        }
        _ => {
            // Transfer SPL
            let amount = gains.token_amount(harvest_token) as u64;
            if amount > 0 {
                spltoken::transfer_from_vault(
                    amount,
                    PDA::liquidation_rewards_vault_from(
                        &ctx.accounts.borrowing_market_state.initial_market_owner,
                    ),
                    &ctx.accounts.liquidation_rewards_to,
                    &ctx.accounts.liquidation_rewards_vault,
                    &ctx.accounts.liquidation_rewards_vault_authority,
                    ctx.accounts.stability_vaults.liquidation_rewards_vault_seed,
                    &ctx.accounts.token_program,
                    ctx.program_id,
                )?;
            }
        }
    };

    // Mint HBB
    if gains.hbb > 0 {
        token_operations::hbb::mint(
            gains.hbb as u64,
            ctx.accounts.borrowing_market_state.hbb_mint_seed,
            ctx.accounts.borrowing_market_state.initial_market_owner,
            ctx.program_id,
            ctx.accounts.hbb_mint.clone(),
            ctx.accounts.hbb_ata.clone(),
            ctx.accounts.hbb_mint_authority.clone(),
            ctx.accounts.token_program.to_account_info(),
        )?;
    }

    epoch_to_scale_to_sum
        .pack_to_zero_copy_account(&mut ctx.accounts.epoch_to_scale_to_sum, LoadingMode::Mut)?;

    Ok(())
}

mod utils {

    use anchor_lang::{
        prelude::{msg, ProgramResult},
        Context, Key, ToAccountInfo,
    };
    use vipers::{assert_ata, assert_keys_eq};

    use crate::key;
    use crate::StabilityToken;

    pub fn assert_permissions(
        ctx: &Context<crate::HarvestLiquidationGains>,
        harvest_token: StabilityToken,
    ) -> ProgramResult {
        let owner = ctx.accounts.stability_provider_state.owner;

        let from_vault = key!(ctx, liquidation_rewards_vault);
        let from_authority = key!(ctx, liquidation_rewards_vault_authority);
        let to_ata = key!(ctx, liquidation_rewards_to);

        let stability_vaults = &ctx.accounts.stability_vaults;
        let borrowing_vaults = &ctx.accounts.borrowing_vaults;

        assert_ata!(ctx.accounts.hbb_ata, owner, ctx.accounts.hbb_mint);

        if harvest_token == StabilityToken::HBB {
            assert_keys_eq!(ctx.accounts.hbb_ata.key, to_ata);
            assert_keys_eq!(ctx.accounts.hbb_mint_authority.key, from_authority);
        } else {
            if harvest_token == StabilityToken::SOL {
                assert_keys_eq!(owner, to_ata);
            } else {
                assert_ata!(
                    to_ata,
                    owner,
                    borrowing_vaults.mint_address_for_stability_token(harvest_token)
                );
            }
            assert_keys_eq!(
                stability_vaults.vault_address(harvest_token),
                from_vault,
                "From vault does not match stability pool liquidation rewards vault"
            );
            assert_keys_eq!(
                stability_vaults.liquidation_rewards_vault_authority,
                from_authority,
                "From vault authority does not match liquidation rewards vaults authority"
            );
        }
        Ok(())
    }
}
