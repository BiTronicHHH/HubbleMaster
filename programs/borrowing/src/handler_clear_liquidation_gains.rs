use crate::{
    borrowing_market::types::ClearLiquidationGainsEffects, key, soltoken,
    stability_pool::liquidations_queue, token_operations::spltoken, utils::pda::PDA,
    CollateralToken,
};
use anchor_lang::prelude::*;

pub fn process(
    ctx: Context<crate::ClearLiquidationGains>,
    token: CollateralToken,
) -> ProgramResult {
    msg!("ix=ClearLiquidationGains");
    // As liquidation events' token transfers cannot be handled atomically
    // we need to clear them by a bot. The bot that runs this event is
    // called the 'clearing agent'. The clearing agent is rewarded for
    // running this instruction.

    // 1. Move funds from collateral vault to rewards vault for x token
    // 2. Move funds from collateral vault to clearing agent ATA
    // 3. Close out pending liquidation events if possible

    utils::assert_permissions(&ctx, token)?;

    let ClearLiquidationGainsEffects {
        clearing_agent_gains,
        stability_pool_gains,
    } = liquidations_queue::clear_liquidation_gains(
        &mut ctx.accounts.liquidations_queue.load_mut()?,
        token,
        key!(ctx, clearing_agent),
        ctx.accounts.clock.unix_timestamp as u64,
    );

    // Transfer collateral from collateral vaults to liquidation reward vaults
    use CollateralToken::*;

    // Transfer SPL collateral from collateral vaults to liquidation reward vaults
    // then transfer from collateral vault to clearing agent ATA
    let amount_to_sp = stability_pool_gains.token_amount(token) as u64;
    let amount_to_ca = clearing_agent_gains.token_amount(token) as u64;

    for (amount, to) in [
        (amount_to_sp, &ctx.accounts.liquidation_rewards_vault),
        (amount_to_ca, &ctx.accounts.clearing_agent_ata),
    ]
    .iter()
    {
        if *amount > 0 {
            // if sol, then liquidation_rewards_vault, collateral_vault, liquidator_ata
            // are all just normal solana accounts
            if token == SOL {
                soltoken::transfer_from_vault(*amount, &ctx.accounts.collateral_vault, to)?;
            } else {
                spltoken::transfer_from_vault(
                    *amount,
                    PDA::collateral_vault_from(
                        &ctx.accounts.borrowing_market_state.initial_market_owner,
                    ),
                    to,
                    &ctx.accounts.collateral_vault,
                    &ctx.accounts.collateral_vaults_authority,
                    ctx.accounts.borrowing_vaults.collateral_vaults_seed,
                    &ctx.accounts.token_program,
                    ctx.program_id,
                )?;
            }
        }
    }

    Ok(())
}

mod utils {

    use anchor_lang::{
        prelude::{msg, ProgramResult},
        Context, Key, ToAccountInfo,
    };
    use vipers::{assert_ata, assert_keys_eq};

    use crate::key;
    use crate::state::CollateralToken;

    pub fn assert_permissions(
        ctx: &Context<crate::ClearLiquidationGains>,
        token: CollateralToken,
    ) -> ProgramResult {
        let clearing_agent = key!(ctx, clearing_agent);
        let clearing_agent_ata = key!(ctx, clearing_agent_ata);

        let from_vault = key!(ctx, collateral_vault);
        let from_authority = key!(ctx, collateral_vaults_authority);
        let to_vault = key!(ctx, liquidation_rewards_vault);

        let borrowing_vaults = &ctx.accounts.borrowing_vaults;
        let stability_vaults = &ctx.accounts.stability_vaults;

        if token == CollateralToken::SOL {
            assert_keys_eq!(clearing_agent, clearing_agent_ata);
        } else {
            assert_ata!(
                clearing_agent_ata,
                clearing_agent,
                borrowing_vaults.mint_address(token)
            );
        }
        assert_keys_eq!(
            borrowing_vaults.vault_address(token),
            from_vault,
            "From vault does not match borrowing market collateral vault"
        );
        assert_keys_eq!(
            borrowing_vaults.collateral_vaults_authority,
            from_authority,
            "From vault authority does not match borrowing market collateral vaults authority"
        );
        assert_keys_eq!(
            stability_vaults.vault_address_for_collateral_token(token),
            to_vault,
            "To vault does not match stability pool liquidation rewards vault"
        );

        Ok(())
    }
}
