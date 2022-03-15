use anchor_lang::prelude::*;
use anchor_spl::token::SetAuthority;
use pda::PDA::*;
use utils::*;

use crate::{
    key,
    stability_pool::stability_pool_operations,
    state::{
        epoch_to_scale_to_sum::{EpochToScaleToSum, LoadingMode},
        CollateralToken,
    },
    utils::pda,
};

pub fn process(ctx: Context<crate::InitializeStabilityPool>) -> ProgramResult {
    msg!("Initializing stability pool!");

    let owner = key!(ctx, initial_market_owner);

    let pda_stability_pool = transfer_stability_pool_to_pda(&ctx, StabilityPool { owner });
    let pda_liq_rewards = pda::make_pda_pubkey(LiquidationsVault { owner }, ctx.program_id);

    transfer_liquidations_vault_to_pda(&ctx, &pda_liq_rewards, CollateralToken::ETH);
    transfer_liquidations_vault_to_pda(&ctx, &pda_liq_rewards, CollateralToken::BTC);
    transfer_liquidations_vault_to_pda(&ctx, &pda_liq_rewards, CollateralToken::SRM);
    transfer_liquidations_vault_to_pda(&ctx, &pda_liq_rewards, CollateralToken::FTT);
    transfer_liquidations_vault_to_pda(&ctx, &pda_liq_rewards, CollateralToken::RAY);

    let stability_pool_state = &mut ctx.accounts.stability_pool_state;

    stability_pool_state.borrowing_market_state = key!(ctx, borrowing_market_state);
    stability_pool_state.epoch_to_scale_to_sum = key!(ctx, epoch_to_scale_to_sum);
    stability_pool_state.liquidations_queue = key!(ctx, liquidations_queue);

    let stability_vaults = &mut ctx.accounts.stability_vaults;

    stability_vaults.stability_pool_state = key!(stability_pool_state);

    stability_vaults.stablecoin_stability_pool_vault = key!(ctx, stablecoin_stability_pool_vault);
    stability_vaults.stablecoin_stability_pool_vault_authority = pda_stability_pool.key;
    stability_vaults.stablecoin_stability_pool_vault_seed = pda_stability_pool.seed;

    stability_vaults.liquidation_rewards_vault_sol = key!(ctx, liquidation_rewards_vault_sol);
    stability_vaults.liquidation_rewards_vault_srm = key!(ctx, liquidation_rewards_vault_srm);
    stability_vaults.liquidation_rewards_vault_eth = key!(ctx, liquidation_rewards_vault_eth);
    stability_vaults.liquidation_rewards_vault_btc = key!(ctx, liquidation_rewards_vault_btc);
    stability_vaults.liquidation_rewards_vault_ray = key!(ctx, liquidation_rewards_vault_ray);
    stability_vaults.liquidation_rewards_vault_ftt = key!(ctx, liquidation_rewards_vault_ftt);

    stability_vaults.liquidation_rewards_vault_authority = pda_liq_rewards.key;
    stability_vaults.liquidation_rewards_vault_seed = pda_liq_rewards.seed;

    // Initialize epoch_to_scale_to_sum
    let epochs = EpochToScaleToSum::default();
    epochs.pack_to_zero_copy_account(&mut ctx.accounts.epoch_to_scale_to_sum, LoadingMode::Init)?;

    // Initialize stability pool state
    let liquidations_queue = &mut ctx.accounts.liquidations_queue;

    stability_pool_operations::initialize_stability_pool(
        stability_pool_state,
        &mut liquidations_queue.load_init()?,
        ctx.accounts.clock.unix_timestamp as u64,
    );

    Ok(())
}

mod utils {
    use anchor_lang::Context;
    use anchor_spl::token;

    use crate::{
        pda::{self, PdaAddress},
        state::CollateralToken,
    };

    pub fn transfer_stability_pool_to_pda(
        ctx: &Context<crate::InitializeStabilityPool>,
        mode: pda::PDA,
    ) -> PdaAddress {
        let authority_pda = pda::make_pda_pubkey(mode, ctx.program_id);

        token::set_authority(
            ctx.accounts.to_set_authority_cpi_context_stability_pool(),
            spl_token::instruction::AuthorityType::AccountOwner,
            Some(authority_pda.key),
        )
        .unwrap();

        authority_pda
    }
    pub fn transfer_liquidations_vault_to_pda(
        ctx: &Context<crate::InitializeStabilityPool>,
        authority_pda: &PdaAddress,
        token: CollateralToken,
    ) {
        token::set_authority(
            ctx.accounts.to_set_authority_cpi_context_liq_rewards(token),
            spl_token::instruction::AuthorityType::AccountOwner,
            Some(authority_pda.key),
        )
        .unwrap();
    }
}

impl<'a, 'b, 'c, 'info> crate::InitializeStabilityPool<'info> {
    pub fn to_set_authority_cpi_context_liq_rewards(
        &self,
        token: CollateralToken,
    ) -> CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>> {
        use CollateralToken::*;
        let cpi_accounts = SetAuthority {
            account_or_mint: match token {
                SRM => self.liquidation_rewards_vault_srm.to_account_info().clone(),
                ETH => self.liquidation_rewards_vault_eth.to_account_info().clone(),
                BTC => self.liquidation_rewards_vault_btc.to_account_info().clone(),
                RAY => self.liquidation_rewards_vault_ray.to_account_info().clone(),
                FTT => self.liquidation_rewards_vault_ftt.to_account_info().clone(),
                _ => unimplemented!(),
            },
            current_authority: self.initial_market_owner.clone(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'a, 'b, 'c, 'info> crate::InitializeStabilityPool<'info> {
    pub fn to_set_authority_cpi_context_stability_pool(
        &self,
    ) -> CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self
                .stablecoin_stability_pool_vault
                .to_account_info()
                .clone(),
            current_authority: self.initial_market_owner.clone(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
