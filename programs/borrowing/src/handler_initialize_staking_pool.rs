use anchor_lang::prelude::*;
use anchor_spl::token::SetAuthority;

use crate::key;
use crate::staking_pool::staking_pool_operations;

pub fn process(
    ctx: Context<crate::InitializeStakingPool>,
    treasury_fee_rate: u16,
) -> ProgramResult {
    let pda_staking_vault = utils::transfer_staking_vault_account_ownership_to_pda(&ctx);

    let staking_pool_state = &mut ctx.accounts.staking_pool_state;

    staking_pool_state.borrowing_market_state = key!(ctx, borrowing_market_state);

    staking_pool_state.staking_vault = key!(ctx, staking_vault);
    staking_pool_state.staking_vault_authority = pda_staking_vault.key;
    staking_pool_state.staking_vault_seed = pda_staking_vault.seed;

    staking_pool_state.treasury_vault = key!(ctx, treasury_vault);
    staking_pool_state.treasury_fee_rate = treasury_fee_rate;

    staking_pool_operations::initialize_staking_pool(staking_pool_state);

    msg!("Done");

    Ok(())
}

impl<'a, 'b, 'c, 'info> crate::InitializeStakingPool<'info> {
    pub fn to_staking_cpi_context(&self) -> CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.staking_vault.clone(),
            current_authority: self.initial_market_owner.clone(),
        };

        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

mod utils {
    use crate::key;
    use crate::pda;
    use anchor_lang::{prelude::msg, Context, ToAccountInfo};
    use anchor_spl::token;

    pub fn transfer_staking_vault_account_ownership_to_pda(
        ctx: &Context<crate::InitializeStakingPool>,
    ) -> pda::PdaAddress {
        let staking_vault_authority_pda = pda::make_pda_pubkey(
            pda::PDA::StakingPool {
                owner: key!(ctx, initial_market_owner),
            },
            ctx.program_id,
        );

        token::set_authority(
            ctx.accounts.to_staking_cpi_context(),
            spl_token::instruction::AuthorityType::AccountOwner,
            Some(staking_vault_authority_pda.key),
        )
        .unwrap();

        msg!(
            "Set staking pool {} to authority {}",
            key!(ctx, staking_vault),
            staking_vault_authority_pda.key
        );

        staking_vault_authority_pda
    }
}
