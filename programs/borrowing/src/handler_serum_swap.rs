use std::num::NonZeroU64;

use crate::borrowing_market::borrowing_operations;
use crate::borrowing_market::types::WithdrawCollateralEffects;
use crate::handler_serum_swap::utils::assert_swap_not_zero;
use crate::log_compute_units;
use crate::state::CollateralToken;
use crate::token_operations::spltoken;
use crate::utils::oracle::get_prices;
use crate::utils::pda::{self, PDA};
use anchor_lang::solana_program::log::sol_log_compute_units;
use anchor_lang::{prelude::*, AccountsClose};
use anchor_spl::dex;
use anchor_spl::dex::serum_dex::{
    instruction::SelfTradeBehavior,
    matching::{OrderType, Side},
    state::MarketState as DexMarketState,
};
use anchor_spl::token;

/// Transfers tokens from the collateral vault to an intermediary account.
/// Afterwards, we place a new order on the Serum DEX Market, with the amount
/// we funded the intermed account.
///
/// All the base tokens/coins (WSOL/BTC/ETH) sent to the ATA of the intermed account
/// will be placed as an order, and we want to get back as much PC/Quote tokens (USDC) as we could.
///
/// # Arguments
///
/// Most of the accounts are Serum market specific - coin vault, pc vault, request queue, event queue, market bids, market asks
///
/// The variables that we are going to send are:
///
/// * `order_payer_token_account` - the account that sends the tokens to the Serum DEX, in the coin vault (intermed_account in our case)
/// * `side` - 0 - ask, we place a sell order, 1 - bid, we place a buy order
/// * `limit_price` - the limit order price we want to buy the quote tokens (USDC) at. We set it to 1
/// because we want to get as much as possible, similar for the pc_quantity
/// * `max_coin_qty` - the max. amount of coins (WSOL/BTC) we want to sell. We determine the lot size of the market
/// and we calculate it accordingly. Currently set to 10^6, the size of the mint's decimals
pub fn process(
    ctx: Context<crate::SerumSwapToUsdc>,
    side: u8,
    base_amount: u64,
    collateral: CollateralToken,
) -> ProgramResult {
    msg!("Ix=SerumSwapToUsdc");
    // Created an intermed account from whom I can send the new order to the serum DEX, the PDA is not a fit for the DEX
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
        collateral_to_transfer_to_user: _,
        close_user_metadata,
    } = borrowing_operations::withdraw_collateral(
        &mut ctx.accounts.borrowing_market_state,
        &mut ctx.accounts.user_metadata,
        base_amount,
        collateral,
        &prices,
    )?;

    let coll_vault_amount_before_transfer =
        token::accessor::amount(&ctx.accounts.collateral_vault)?;
    let usdc_wallet_amount_before_transfer = token::accessor::amount(&ctx.accounts.pc_wallet)?;
    let dex_swap_user_amount_before_transfer =
        token::accessor::amount(&ctx.accounts.dex_swap_account)?;

    msg!(
        "coll_vault_amount_before_transfer {}",
        coll_vault_amount_before_transfer
    );
    msg!(
        "usdc_wallet_amount_before_transfer {}",
        usdc_wallet_amount_before_transfer
    );
    msg!(
        "dex_swap_user_amount_before_transfer {}",
        dex_swap_user_amount_before_transfer
    );

    spltoken::transfer_from_vault(
        base_amount,
        PDA::collateral_vault_from(&ctx.accounts.borrowing_market_state.initial_market_owner),
        &ctx.accounts.dex_swap_account,
        &ctx.accounts.collateral_vault,
        &ctx.accounts.collateral_from_authority,
        ctx.accounts.borrowing_vaults.collateral_vaults_seed,
        &ctx.accounts.token_program,
        &ctx.program_id,
    )?;

    log_compute_units!("Serum Swap - After Transfer");

    if close_user_metadata {
        ctx.accounts
            .user_metadata
            .close(ctx.accounts.owner.clone())?;
    }

    let dex_accs = dex::NewOrderV3 {
        market: ctx.accounts.market.clone(),
        open_orders: ctx.accounts.open_orders.clone(),
        request_queue: ctx.accounts.request_queue.clone(),
        event_queue: ctx.accounts.event_queue.clone(),
        market_bids: ctx.accounts.bids.clone(),
        market_asks: ctx.accounts.asks.clone(),
        order_payer_token_account: ctx.accounts.dex_swap_account.clone(),
        open_orders_authority: ctx.accounts.owner.clone(),
        coin_vault: ctx.accounts.coin_vault.clone(),
        pc_vault: ctx.accounts.pc_vault.clone(),
        token_program: ctx.accounts.token_program.clone(),
        rent: ctx.accounts.rent.to_account_info().clone(),
    };

    let ctx_order = CpiContext::new(ctx.accounts.dex_program.clone(), dex_accs);

    // we're generally selling (0 - ask), but kept it in here for further use

    let side = match side {
        0 => Side::Ask,
        1 => Side::Bid,
        _ => panic!("wrong side"),
    };

    // Limit price = 1, I want to buy as much PC as I could

    let limit_price = 1;
    let max_pc_qty = u64::MAX;
    let max_coin_qty = {
        let dex_market =
            DexMarketState::load(&ctx.accounts.market, &ctx.accounts.dex_program.key())?;
        base_amount.checked_div(dex_market.coin_lot_size).unwrap()
    };

    let coll_vault_amount_before_swap = token::accessor::amount(&ctx.accounts.collateral_vault)?;
    let usdc_wallet_amount_before_swap = token::accessor::amount(&ctx.accounts.pc_wallet)?;
    let dex_swap_user_amount_before_swap = token::accessor::amount(&ctx.accounts.dex_swap_account)?;

    msg!(
        "coll_vault_amount_before_swap {}",
        coll_vault_amount_before_swap
    );
    msg!(
        "usdc_wallet_amount_before_swap {}",
        usdc_wallet_amount_before_swap
    );
    msg!(
        "dex_swap_user_amount_before_swap {}",
        dex_swap_user_amount_before_swap
    );

    dex::new_order_v3(
        ctx_order,
        side.into(),
        NonZeroU64::new(limit_price).unwrap(),
        NonZeroU64::new(max_coin_qty).unwrap(),
        NonZeroU64::new(max_pc_qty).unwrap(),
        SelfTradeBehavior::DecrementTake,
        OrderType::ImmediateOrCancel,
        0,     // ok to hardcode this (only used for cancels)
        65535, // dex's custom compute budget parameter
    )?;

    log_compute_units!("Serum Swap - After New Order");

    let settle_accs = dex::SettleFunds {
        market: ctx.accounts.market.clone(),
        open_orders: ctx.accounts.open_orders.clone(),
        open_orders_authority: ctx.accounts.owner.clone(),
        coin_vault: ctx.accounts.coin_vault.clone(),
        pc_vault: ctx.accounts.pc_vault.clone(),
        coin_wallet: ctx.accounts.collateral_vault.clone(),
        pc_wallet: ctx.accounts.pc_wallet.clone(),
        vault_signer: ctx.accounts.vault_signer.clone(),
        token_program: ctx.accounts.token_program.clone(),
    };

    let ctx_settle = CpiContext::new(ctx.accounts.dex_program.clone(), settle_accs);

    dex::settle_funds(ctx_settle)?;

    log_compute_units!("Serum Swap - After Settle");

    let coll_vault_amount_after = token::accessor::amount(&ctx.accounts.collateral_vault)?;
    let usdc_wallet_amount_after = token::accessor::amount(&ctx.accounts.pc_wallet)?;
    let dex_swap_user_amount_after = token::accessor::amount(&ctx.accounts.dex_swap_account)?;

    msg!("coll_vault_amount_after {}", coll_vault_amount_after);
    msg!("usdc_wallet_amount_after {}", usdc_wallet_amount_after);
    msg!("dex_swap_user_amount_after {}", dex_swap_user_amount_after);

    // Verify that the user received usdc and that the dex_swap intermed account doesn't hold any coins
    let diff_usdc = usdc_wallet_amount_after
        .checked_sub(usdc_wallet_amount_before_swap)
        .unwrap();
    let diff_base_user = dex_swap_user_amount_before_swap
        .checked_sub(dex_swap_user_amount_after)
        .unwrap();
    let diff_base_vault = coll_vault_amount_before_transfer
        .checked_sub(coll_vault_amount_after)
        .unwrap();

    assert_swap_not_zero(diff_usdc, diff_base_user, diff_base_vault)?;

    Ok(())
}

mod utils {
    use std::cell::RefMut;

    use anchor_lang::{
        prelude::{msg, AccountInfo, ProgramError, ProgramResult, Pubkey},
        Context, Key,
    };
    use anchor_spl::dex;
    use anchor_spl::dex::serum_dex::state::{Market as DexMarket, ToAlignedBytes};
    use vipers::{assert_ata, assert_keys_eq};

    use crate::{BorrowError, CollateralToken};

    pub fn assert_permissions(
        ctx: &Context<crate::SerumSwapToUsdc>,
        collateral: CollateralToken,
    ) -> ProgramResult {
        let borrowing_vaults = &ctx.accounts.borrowing_vaults;

        let from_vault = ctx.accounts.collateral_vault.key;
        let from_authority = ctx.accounts.collateral_from_authority.key;

        assert_ata!(
            ctx.accounts.pc_wallet,
            ctx.accounts.user_metadata.owner,
            ctx.accounts.usdc_mint
        );

        assert_keys_eq!(ctx.accounts.dex_program.key, dex::ID);

        if from_vault != &borrowing_vaults.vault_address(collateral) {
            msg!(
                "From vault does not match borrowing vaults {:} vs {:}",
                from_vault,
                &borrowing_vaults.vault_address(collateral)
            );
            return Err(ProgramError::IllegalOwner);
        }

        if collateral != CollateralToken::SOL {
            if from_authority != &borrowing_vaults.collateral_vaults_authority {
                msg!(
                    "From vault authority does not match borrowing vaults authority {:?} vs {:?}",
                    from_authority,
                    &borrowing_vaults.collateral_vaults_authority
                );
                return Err(ProgramError::IllegalOwner);
            }
        }

        let expected_base_mint = &ctx.accounts.borrowing_vaults.mint_address(collateral);

        assert_dex_inputs(
            &ctx.accounts.market,
            &ctx.accounts.dex_program.key,
            expected_base_mint,
            &ctx.accounts.usdc_mint.key,
        )?;

        Ok(())
    }

    pub fn assert_dex_inputs(
        market: &AccountInfo,
        dex_program: &Pubkey,
        expected_base_mint: &Pubkey,
        expected_quote_mint: &Pubkey,
    ) -> ProgramResult {
        let market_state = DexMarket::load(market, dex_program)?;
        let market_v1 = match market_state {
            DexMarket::V1(v1) => v1,
            DexMarket::V2(v2) => RefMut::map(v2, |m| &mut m.inner),
        };

        let expected_base_mint = expected_base_mint.to_aligned_bytes();
        let expected_quote_mint = expected_quote_mint.to_aligned_bytes();

        if { market_v1.coin_mint } != expected_base_mint || { market_v1.pc_mint }
            != expected_quote_mint
        {
            return Err(BorrowError::InvalidDexInputs.into());
        }

        Ok(())
    }

    pub fn assert_swap_not_zero(
        diff_usdc: u64,
        diff_base_user: u64,
        diff_base_vault: u64,
    ) -> ProgramResult {
        if diff_usdc == 0 {
            return Err(BorrowError::ZeroSwap.into());
        }

        if diff_base_user == 0 {
            return Err(BorrowError::ZeroSwap.into());
        }

        if diff_base_vault == 0 {
            return Err(BorrowError::ZeroSwap.into());
        }

        Ok(())
    }
}
