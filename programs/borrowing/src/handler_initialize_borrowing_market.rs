use crate::{
    borrowing_market::borrowing_operations, key, pda, state::CollateralToken,
    utils::consts::BOOTSTRAP_PERIOD,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, SetAuthority};
use pda::PDA::*;
use spl_token::instruction::AuthorityType::*;
use utils::*;

pub fn process(ctx: Context<crate::InitializeBorrowingMarket>) -> ProgramResult {
    msg!("Initializing market!");

    assert_permissions(&ctx)?;

    let owner = key!(ctx, initial_market_owner);

    let pda_mint_stable = transfer_to_pda(&ctx, StablecoinMint { owner }, MintTokens);
    let pda_mint_hbb = transfer_to_pda(&ctx, HbbMint { owner }, MintTokens);
    let pda_borrow = transfer_to_pda(&ctx, BorrowingFeesAccount { owner }, AccountOwner);
    let pda_burning = transfer_to_pda(&ctx, BurningPotAccount { owner }, AccountOwner);

    use CollateralToken::*;
    let pda_coll_vault = pda::make_pda_pubkey(CollateralVault { owner }, ctx.program_id);
    transfer_to_pda_collateral_vault(&ctx, ETH, &pda_coll_vault);
    transfer_to_pda_collateral_vault(&ctx, BTC, &pda_coll_vault);
    transfer_to_pda_collateral_vault(&ctx, SRM, &pda_coll_vault);
    transfer_to_pda_collateral_vault(&ctx, RAY, &pda_coll_vault);
    transfer_to_pda_collateral_vault(&ctx, FTT, &pda_coll_vault);

    let global_config = &mut ctx.accounts.global_config;
    global_config.version = 0;
    global_config.initial_market_owner = key!(ctx, initial_market_owner);
    global_config.is_borrowing_allowed = true;
    global_config.borrow_limit_usdh = 1_000;

    // 6. Initialize Global State
    let market = &mut ctx.accounts.borrowing_market_state;
    market.initial_market_owner = key!(ctx, initial_market_owner);
    market.redemptions_queue = key!(ctx, redemptions_queue);

    market.stablecoin_mint = key!(ctx, stablecoin_mint);
    market.stablecoin_mint_authority = pda_mint_stable.key;
    market.stablecoin_mint_seed = pda_mint_stable.seed;

    market.hbb_mint = key!(ctx, hbb_mint);
    market.hbb_mint_authority = pda_mint_hbb.key;
    market.hbb_mint_seed = pda_mint_hbb.seed;

    let borrowing_vaults = &mut ctx.accounts.borrowing_vaults;

    // Borrowing & Burning vaults
    borrowing_vaults.borrowing_market_state = key!(market);
    borrowing_vaults.borrowing_fees_vault = key!(ctx, borrowing_fees_vault);
    borrowing_vaults.borrowing_fees_vault_authority = pda_borrow.key;
    borrowing_vaults.borrowing_fees_vault_seed = pda_borrow.seed;
    borrowing_vaults.burning_vault = key!(ctx, burning_vault);
    borrowing_vaults.burning_vault_authority = pda_burning.key;
    borrowing_vaults.burning_vault_seed = pda_burning.seed;

    // Collateral vaults
    borrowing_vaults.collateral_vault_sol = key!(ctx, collateral_vault_sol);
    borrowing_vaults.collateral_vault_srm = key!(ctx, collateral_vault_srm);
    borrowing_vaults.collateral_vault_eth = key!(ctx, collateral_vault_eth);
    borrowing_vaults.collateral_vault_btc = key!(ctx, collateral_vault_btc);
    borrowing_vaults.collateral_vault_ray = key!(ctx, collateral_vault_ray);
    borrowing_vaults.collateral_vault_ftt = key!(ctx, collateral_vault_ftt);

    borrowing_vaults.collateral_vaults_authority = pda_coll_vault.key;
    borrowing_vaults.collateral_vaults_seed = pda_coll_vault.seed;

    borrowing_vaults.srm_mint = key!(ctx, srm_mint);
    borrowing_vaults.eth_mint = key!(ctx, eth_mint);
    borrowing_vaults.btc_mint = key!(ctx, btc_mint);
    borrowing_vaults.ray_mint = key!(ctx, ray_mint);
    borrowing_vaults.ftt_mint = key!(ctx, ftt_mint);

    // Update state
    let now = ctx.accounts.clock.unix_timestamp as u64;
    borrowing_operations::initialize_borrowing_market(market, now + BOOTSTRAP_PERIOD);

    Ok(())
}

impl<'a, 'b, 'c, 'info> crate::InitializeBorrowingMarket<'info> {
    pub fn to_set_authority_cpi_context(
        &self,
        account_to_context: pda::PDA,
    ) -> CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>> {
        use pda::PDA::*;
        let cpi_accounts = SetAuthority {
            account_or_mint: match account_to_context {
                BorrowingFeesAccount { .. } => self.borrowing_fees_vault.to_account_info().clone(),
                BurningPotAccount { .. } => self.burning_vault.to_account_info().clone(),
                StablecoinMint { .. } => self.stablecoin_mint.to_account_info().clone(),
                HbbMint { .. } => self.hbb_mint.to_account_info().clone(),
                _ => unimplemented!(),
            },
            current_authority: self.initial_market_owner.clone(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
    pub fn to_set_authority_cpi_context_coll_vault(
        &self,
        token: CollateralToken,
    ) -> CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: match token {
                CollateralToken::SRM => self.collateral_vault_srm.to_account_info().clone(),
                CollateralToken::ETH => self.collateral_vault_eth.to_account_info().clone(),
                CollateralToken::BTC => self.collateral_vault_btc.to_account_info().clone(),
                CollateralToken::RAY => self.collateral_vault_ray.to_account_info().clone(),
                CollateralToken::FTT => self.collateral_vault_ftt.to_account_info().clone(),
                _ => unimplemented!(),
            },
            current_authority: self.initial_market_owner.clone(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

mod utils {
    use crate::pda::PdaAddress;

    pub fn assert_permissions(ctx: &Context<crate::InitializeBorrowingMarket>) -> ProgramResult {
        // 5. Validate collateral pool
        if ctx.accounts.collateral_vault_sol.owner != ctx.program_id {
            return Err(ProgramError::IllegalOwner);
        }

        Ok(())
    }

    use super::*;
    pub fn transfer_to_pda_collateral_vault(
        ctx: &Context<crate::InitializeBorrowingMarket>,
        token: CollateralToken,
        authority_pda: &PdaAddress,
    ) {
        token::set_authority(
            ctx.accounts.to_set_authority_cpi_context_coll_vault(token),
            spl_token::instruction::AuthorityType::AccountOwner,
            Some(authority_pda.key),
        )
        .unwrap();
    }
    pub fn transfer_to_pda(
        ctx: &Context<crate::InitializeBorrowingMarket>,
        mode: pda::PDA,
        authority_type: spl_token::instruction::AuthorityType,
    ) -> PdaAddress {
        let authority_pda = pda::make_pda_pubkey(mode, ctx.program_id);

        token::set_authority(
            ctx.accounts.to_set_authority_cpi_context(mode),
            authority_type,
            Some(authority_pda.key),
        )
        .unwrap();

        authority_pda
    }
}
