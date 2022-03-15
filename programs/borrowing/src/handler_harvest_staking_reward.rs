use crate::staking_pool::staking_pool_operations;
use crate::staking_pool::types::HarvestEffects;
use crate::token_operations::stablecoin;
use anchor_lang::prelude::*;

pub fn process(ctx: Context<crate::HarvestRewardStakingPool>) -> ProgramResult {
    utils::assert_permissions(&ctx)?;

    let borrowing_market_state = &ctx.accounts.borrowing_market_state;
    let borrowing_vaults = &ctx.accounts.borrowing_vaults;
    let staking_pool_state = &mut ctx.accounts.staking_pool_state;
    let user_staking_state = &mut ctx.accounts.user_staking_state;

    let HarvestEffects { reward } =
        staking_pool_operations::user_harvest(staking_pool_state, user_staking_state)?;

    msg!("Reward is {}", reward);
    if reward > 0 {
        stablecoin::transfer_from_borrowing_fees_vault(
            reward as u64,
            borrowing_market_state.initial_market_owner,
            &ctx.accounts.user_stablecoin_rewards_ata,
            &ctx.accounts.borrowing_fees_vault,
            &ctx.accounts.borrowing_fees_vault_authority,
            borrowing_vaults.borrowing_fees_vault_seed,
            &ctx.accounts.token_program,
            ctx.program_id,
        )?;
    }

    Ok(())
}

mod utils {
    use anchor_lang::{
        prelude::{msg, ProgramResult},
        Context,
    };
    use vipers::assert_ata;

    use crate::BorrowError;

    pub fn assert_permissions(ctx: &Context<crate::HarvestRewardStakingPool>) -> ProgramResult {
        assert_amount_not_zero(ctx.accounts.user_staking_state.user_stake)?;

        assert_there_is_reward(
            ctx.accounts.user_staking_state.rewards_tally,
            ctx.accounts.user_staking_state.user_stake,
            ctx.accounts.staking_pool_state.reward_per_token,
        )?;

        assert_ata!(
            ctx.accounts.user_stablecoin_rewards_ata,
            ctx.accounts.owner,
            ctx.accounts.borrowing_market_state.stablecoin_mint,
        );

        Ok(())
    }

    pub fn assert_amount_not_zero(amount: u128) -> ProgramResult {
        if amount == 0 {
            Err(BorrowError::NothingStaked.into())
        } else {
            Ok(())
        }
    }

    fn assert_there_is_reward(
        rewards_tally: u128,
        amount_staked: u128,
        reward_per_token: u128,
    ) -> ProgramResult {
        if rewards_tally
            >= (amount_staked as u128)
                .checked_mul(reward_per_token)
                .unwrap()
        {
            Err(BorrowError::NoRewardToWithdraw.into())
        } else {
            Ok(())
        }
    }
}
