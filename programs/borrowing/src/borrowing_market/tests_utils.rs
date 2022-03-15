#[cfg(test)]
pub mod utils {
    use std::cell::RefCell;

    use crate::borrowing_market::borrowing_operations::utils::set_addresses;
    use crate::borrowing_market::types::BorrowStablecoinEffects;
    use crate::state::epoch_to_scale_to_sum::EpochToScaleToSum;
    use crate::state::StabilityPoolState;
    use crate::utils::consts::{
        BTC_DECIMALS, ETH_DECIMALS, FTT_DECIMALS, RAY_DECIMALS, SOL_DECIMALS, SRM_DECIMALS,
        USDH_DECIMALS,
    };
    use crate::utils::finance::CollateralInfo;
    use crate::{
        borrowing_market::borrowing_operations, BorrowingMarketState, LiquidationsQueue,
        StakingPoolState, UserMetadata,
    };
    use crate::{CollateralAmounts, CollateralToken, Price, TokenPrices};
    use anchor_lang::prelude::Pubkey;
    use decimal_wad::{common::TryMul, decimal::Decimal};

    #[macro_export]
    macro_rules! deposited {
        ($user:ident, $coll: ident) => {
            $user.deposited_collateral.token_amount($coll) as u64
        };

        ($user:expr, $coll: expr) => {
            $user.deposited_collateral.token_amount($coll) as u64
        };

        ($borrowing_market_state: ident, $coll: expr) => {
            $borrowing_market_state
                .deposited_collateral
                .token_amount($coll) as u64
        };
    }

    pub fn new_borrower(
        market: &mut BorrowingMarketState,
        staking_pool: &mut StakingPoolState,
        deposit: u64,
        borrow: u64,
        prices: &TokenPrices,
        now: u64,
    ) -> (UserMetadata, BorrowStablecoinEffects) {
        let mut user = UserMetadata::default();
        borrowing_operations::approve_trove(market, &mut user).unwrap();
        borrowing_operations::deposit_collateral(market, &mut user, deposit, CollateralToken::SOL)
            .unwrap();

        let effects = borrowing_operations::borrow_stablecoin(
            market,
            &mut user,
            staking_pool,
            borrow,
            &prices,
            now,
        )
        .unwrap();

        (user, effects)
    }

    pub fn new_borrowing_users(
        borrowing_market_state: &mut BorrowingMarketState,
        staking_pool_state: &mut StakingPoolState,
        count: usize,
        borrow_amount: u64,
        deposit_collateral: u64,
        now_timestamp: u64,
    ) -> Vec<UserMetadata> {
        new_borrowing_users_with_price(
            borrowing_market_state,
            staking_pool_state,
            count,
            borrow_amount,
            deposit_collateral,
            40.0,
            now_timestamp,
        )
    }

    pub fn new_borrowing_users_with_price(
        borrowing_market_state: &mut BorrowingMarketState,
        staking_pool_state: &mut StakingPoolState,
        count: usize,
        borrow_amount: u64,
        deposit_collateral: u64,
        price: f64,
        now_timestamp: u64,
    ) -> Vec<UserMetadata> {
        let collaterals: Vec<CollateralAmounts> = vec![deposit_collateral; count]
            .iter()
            .map(|amount_sol| CollateralAmounts::of_token(*amount_sol, CollateralToken::SOL))
            .collect();
        let borrow_amounts = vec![borrow_amount; count];
        new_borrowing_users_with_amounts_and_price(
            borrowing_market_state,
            staking_pool_state,
            count,
            &borrow_amounts,
            &collaterals,
            price,
            now_timestamp,
        )
    }

    pub fn new_borrowing_users_with_amounts(
        borrowing_market_state: &mut BorrowingMarketState,
        staking_pool_state: &mut StakingPoolState,
        count: usize,
        borrow_amounts: &[u64],
        deposit_collateral: &[CollateralAmounts],
        now_timestamp: u64,
    ) -> Vec<UserMetadata> {
        new_borrowing_users_with_amounts_and_price(
            borrowing_market_state,
            staking_pool_state,
            count,
            borrow_amounts,
            deposit_collateral,
            40.0,
            now_timestamp,
        )
    }

    pub fn new_borrowing_users_with_amounts_and_price(
        borrowing_market_state: &mut BorrowingMarketState,
        staking_pool_state: &mut StakingPoolState,
        count: usize,
        borrow_amount: &[u64],
        deposit_collateral: &[CollateralAmounts],
        price: f64,
        now_timestamp: u64,
    ) -> Vec<UserMetadata> {
        (0..count)
            .map(|i| {
                let mut user = UserMetadata::default();

                let user_pk = Pubkey::new_unique();
                let user_metadata_pk = Pubkey::new_unique();
                borrowing_operations::approve_trove(borrowing_market_state, &mut user).unwrap();

                set_addresses(&mut user, user_pk, user_metadata_pk);

                use CollateralToken::*;
                for token in [SOL, ETH, SRM, FTT, BTC, RAY] {
                    let amount = deposit_collateral[i].token_amount(token);
                    if amount > 0 {
                        borrowing_operations::deposit_collateral(
                            borrowing_market_state,
                            &mut user,
                            amount as u64,
                            token,
                        )
                        .unwrap();
                    }
                }

                let BorrowStablecoinEffects {
                    amount_mint_to_fees_vault,
                    amount_mint_to_user,
                    amount_mint_to_treasury_vault,
                } = borrowing_operations::borrow_stablecoin(
                    borrowing_market_state,
                    &mut user,
                    staking_pool_state,
                    borrow_amount[i],
                    &TokenPrices {
                        sol: Price::from_f64(price, CollateralToken::SOL),
                        eth: Price::from_f64(price, CollateralToken::ETH),
                        btc: Price::from_f64(price, CollateralToken::BTC),
                        srm: Price::from_f64(price, CollateralToken::SRM),
                        ray: Price::from_f64(price, CollateralToken::RAY),
                        ftt: Price::from_f64(price, CollateralToken::FTT),
                    },
                    now_timestamp,
                )
                .unwrap();

                user.borrowed_stablecoin =
                    amount_mint_to_user + amount_mint_to_fees_vault + amount_mint_to_treasury_vault;
                user.deposited_collateral = deposit_collateral[i];
                user
            })
            .collect()
    }

    pub fn set_up_market() -> (
        BorrowingMarketState,
        StabilityPoolState,
        EpochToScaleToSum,
        RefCell<LiquidationsQueue>,
        StakingPoolState,
        u64,
    ) {
        let mut market = BorrowingMarketState::new();
        let mut stability_pool_state = StabilityPoolState::default();
        let mut epoch_to_scale_to_sum = EpochToScaleToSum::default();
        let liquidations = RefCell::new(LiquidationsQueue::default());
        let mut staking_pool_state = StakingPoolState {
            treasury_fee_rate: 1_500,
            ..Default::default()
        };

        let now_timestamp = 0;

        borrowing_operations::initialize_borrowing_market(&mut market, 0);

        (
            market,
            stability_pool_state,
            epoch_to_scale_to_sum,
            liquidations,
            staking_pool_state,
            now_timestamp,
        )
    }

    pub fn calculate_max_withdrawable(
        prices: &TokenPrices,
        user: &UserMetadata,
        token: CollateralToken,
    ) -> u64 {
        // Calculate collateral value for all the other coins
        // Calculate min_collateral_necessary for `asset` such that it's within MCR
        // Max withdrawable is amount_in_asset - min_collateral_necessary

        let deposited_collateral = user.deposited_collateral;

        let asset_amount = deposited_collateral.token_amount(token);
        let asset_price = prices.token_amount(token);

        let deposited_collateral_in_token = CollateralAmounts::of_token(asset_amount, token);
        let deposited_collateral_excluding_token =
            deposited_collateral.sub(&deposited_collateral_in_token);

        // Calculate collateral value for all the other coins
        let collateral_excluding_token_in_usd =
            CollateralInfo::calc_market_value_usdh(&prices, &deposited_collateral_excluding_token);

        // Calculate collateral value for the given coin only
        let collateral_in_token_in_usd =
            CollateralInfo::calc_market_value_usdh(&prices, &deposited_collateral_in_token);

        // 110%
        let min_collateral_usd = Decimal::from(user.borrowed_stablecoin)
            .try_mul(Decimal::from_percent(110))
            .unwrap()
            .try_floor_u64()
            .unwrap();

        let necessary_collateral_in_token_in_usd =
            u64::max(0, min_collateral_usd - collateral_excluding_token_in_usd);

        let surplus_collateral_in_token_in_usd =
            collateral_in_token_in_usd - necessary_collateral_in_token_in_usd;

        let remaining_collateral_in_asset =
            calc_amount_given_price(surplus_collateral_in_token_in_usd, &asset_price, token);

        // msg!(
        //     "{}, {}, {}",
        //     deposited_lamports,
        //     borrowed_stablecoin,
        //     min_collateral_stablecoin
        // );
        remaining_collateral_in_asset
    }

    pub fn calc_amount_given_price(
        market_value: u64,
        price: &Price,
        collateral: CollateralToken,
    ) -> u64 {
        let token_decimals = match collateral {
            CollateralToken::SOL => SOL_DECIMALS,
            CollateralToken::ETH => ETH_DECIMALS,
            CollateralToken::BTC => BTC_DECIMALS,
            CollateralToken::SRM => SRM_DECIMALS,
            CollateralToken::RAY => RAY_DECIMALS,
            CollateralToken::FTT => FTT_DECIMALS,
        };

        // let market_value = (amount as u128) * (price.value as u128)
        //     / 10_u128.pow((token_decimals + price.exp - USDH_DECIMALS) as u32);
        // market_value = amount * px / pow
        // amount = market_value * pow / px;
        ((market_value as u128) * 10_u128.pow((token_decimals + price.exp - USDH_DECIMALS) as u32)
            / (price.value as u128)) as u64
    }
}
