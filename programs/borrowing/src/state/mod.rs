use anchor_lang::prelude::*;
use anchor_lang::zero_copy;
use borsh::{BorshDeserialize, BorshSerialize};
use num_derive::FromPrimitive;
use struct_arithmetic::StructArithmetic;

mod borrowing_market_state;
mod borrowing_vaults;
mod collateral_amounts;
mod deposit_snapshot;
pub mod epoch_to_scale_to_sum;
mod liquidations_queue;
pub mod redemptions_queue;
mod stability_collateral_amounts;
mod stability_pool_state;
mod stability_provider_state;
mod stability_token_map;
mod stability_vaults;
mod staking_pool_state;
mod token_map;
mod user_staking_state;

#[account]
#[derive(Debug)]
pub struct GlobalConfig {
    pub version: u8,
    pub initial_market_owner: Pubkey,
    pub is_borrowing_allowed: bool,
    pub borrow_limit_usdh: u64,
    _padding: [u8; 1024],
}

impl Default for GlobalConfig {
    fn default() -> GlobalConfig {
        GlobalConfig {
            version: 0,
            initial_market_owner: Pubkey::new(&[0; 32]),
            is_borrowing_allowed: false,
            borrow_limit_usdh: 0,
            _padding: [0; 1024],
        }
    }
}

#[derive(FromPrimitive, PartialEq, Eq, Clone, Copy)]
pub enum GlobalConfigOption {
    IsBorrowingAllowed = 0,
    BorrowLimitUsdh = 1,
}

#[account]
#[derive(Debug, Default)]
pub struct BorrowingMarketState {
    pub version: u8,

    // Global admin, needed for seed generation
    pub initial_market_owner: Pubkey,

    // Global state
    pub redemptions_queue: Pubkey,

    // Mint Account from which stablecoin is minted (owned by program PDA)
    // Authority which can MINT tokens out of stablecoin_mint
    pub stablecoin_mint: Pubkey,
    pub stablecoin_mint_authority: Pubkey,
    pub stablecoin_mint_seed: u8,

    // Mint Account from which HBB is minted (owned by program PDA)
    // Authority which can MINT & Burn tokens out of hbb_mint
    pub hbb_mint: Pubkey,
    pub hbb_mint_authority: Pubkey,
    pub hbb_mint_seed: u8,

    // State
    pub num_users: u64,
    pub num_active_users: u64,
    pub stablecoin_borrowed: u64,
    pub deposited_collateral: CollateralAmounts,
    pub inactive_collateral: CollateralAmounts,

    // First two weeks of the protocol being live
    pub bootstrap_period_timestamp: u64,

    // bps
    pub base_rate_bps: u16,
    pub last_fee_event: u64,

    // Redistribution data
    // During liquidations, when there is no stability in the
    // Stability pool, we redistribute the collateral and the
    // debt proportionately among all users
    pub redistributed_stablecoin: u64,

    pub total_stake: u64,
    pub collateral_reward_per_token: TokenMap,
    pub stablecoin_reward_per_token: u128,

    // As of last liquidation
    pub total_stake_snapshot: u64,
    pub borrowed_stablecoin_snapshot: u64,
}

#[account]
#[derive(Debug, Default)]
pub struct BorrowingVaults {
    // Borrowing market the vaults belong to
    pub borrowing_market_state: Pubkey,

    // Burning pot where paid debts are sent, (owned by program seed)
    // Pda which owns the burning pot
    pub burning_vault: Pubkey,
    pub burning_vault_authority: Pubkey,
    pub burning_vault_seed: u8,

    // Stablecoin account where fees are sent, (owned by program seed)
    // Pda which owns the borrowing fees pot
    pub borrowing_fees_vault: Pubkey,
    pub borrowing_fees_vault_authority: Pubkey,
    pub borrowing_fees_vault_seed: u8,

    // Account where collateral is stored
    pub collateral_vault_sol: Pubkey,
    pub collateral_vault_srm: Pubkey,
    pub collateral_vault_eth: Pubkey,
    pub collateral_vault_btc: Pubkey,
    pub collateral_vault_ray: Pubkey,
    pub collateral_vault_ftt: Pubkey,

    // One authority for all collateral vaults
    pub collateral_vaults_authority: Pubkey,
    pub collateral_vaults_seed: u8,

    pub srm_mint: Pubkey,
    pub eth_mint: Pubkey,
    pub btc_mint: Pubkey,
    pub ray_mint: Pubkey,
    pub ftt_mint: Pubkey,
}

#[account]
#[derive(Debug, Default)]
pub struct StabilityPoolState {
    // Borrowing market the pool belongs to
    pub borrowing_market_state: Pubkey,

    pub epoch_to_scale_to_sum: Pubkey,
    pub liquidations_queue: Pubkey,

    // Data state
    pub version: u8,
    pub num_users: u64,
    pub total_users_providing_stability: u64,
    pub stablecoin_deposited: u64,
    pub hbb_emissions_start_ts: u64,

    // Gains
    pub cumulative_gains_total: StabilityTokenMap,
    pub pending_collateral_gains: StabilityTokenMap,
    pub current_epoch: u64,
    pub current_scale: u64,
    pub p: u128,

    // Precision errors
    pub last_stablecoin_loss_error_offset: u64,
    pub last_coll_loss_error_offset: StabilityCollateralAmounts,
}

#[account]
#[derive(Debug, Default)]
pub struct StabilityVaults {
    pub stability_pool_state: Pubkey,

    // Where the users lock in their coins to absorb liquidations
    pub stablecoin_stability_pool_vault: Pubkey,
    pub stablecoin_stability_pool_vault_authority: Pubkey,
    pub stablecoin_stability_pool_vault_seed: u8,

    // Account where collateral is stored
    pub liquidation_rewards_vault_sol: Pubkey,
    pub liquidation_rewards_vault_srm: Pubkey,
    pub liquidation_rewards_vault_eth: Pubkey,
    pub liquidation_rewards_vault_btc: Pubkey,
    pub liquidation_rewards_vault_ray: Pubkey,
    pub liquidation_rewards_vault_ftt: Pubkey,

    pub liquidation_rewards_vault_authority: Pubkey,
    pub liquidation_rewards_vault_seed: u8,
}

#[account]
#[derive(Debug, Default)]
pub struct UserMetadata {
    // due to zero_copy we can't make this an enum
    // 0 - inactive
    // 1 - active
    // 2 - liquidated
    pub version: u8,
    pub status: u8,
    pub user_id: u64,
    pub metadata_pk: Pubkey,
    pub owner: Pubkey,
    pub borrowing_market_state: Pubkey,
    pub stablecoin_ata: Pubkey,

    // This is collateral deposited (living in the collateral vaults),
    // belonging to the user without user being a borrower (or not yet)
    // - could be a redeemer
    // - could be a bot (filler, clearer)
    // - could be a user without debt, that just started depositing without borrowing
    // We need to make the distinction to avoid making this collateral part of the
    // global collateral ratio. Can be withdrawn without any issues.
    // When the user does borrow this becomes valid for CR calculations.
    // This is especially important since, during redistributions
    // a zero debt account is useless.
    pub inactive_collateral: CollateralAmounts,

    // Borrowing
    pub deposited_collateral: CollateralAmounts,
    pub borrowed_stablecoin: u64,

    // Redistribution
    pub user_stake: u64,
    pub user_collateral_reward_per_token: TokenMap,
    pub user_stablecoin_reward_per_token: u128,
}

#[account]
#[derive(Debug, Default)]
pub struct StakingPoolState {
    // Borrowing market the pool belongs to
    pub borrowing_market_state: Pubkey,

    // Metadata used for analytics
    pub total_distributed_rewards: u128,
    pub rewards_not_yet_claimed: u128,

    // Data used to calculate the rewards of the user
    pub version: u8,
    pub num_users: u64,
    pub total_users_providing_stability: u64,
    pub total_stake: u128,
    pub reward_per_token: u128,
    pub prev_reward_loss: u128,

    pub staking_vault: Pubkey,
    pub staking_vault_authority: Pubkey,
    pub staking_vault_seed: u8,

    pub treasury_vault: Pubkey,
    pub treasury_fee_rate: u16,
}

#[account]
#[derive(Debug, Default)]
pub struct UserStakingState {
    pub version: u8,
    pub user_id: u64,
    pub staking_pool_state: Pubkey,
    pub owner: Pubkey,

    // User data to account for rewards
    pub user_stake: u128,
    pub rewards_tally: u128,
}

#[account]
#[derive(Debug, Default)]
pub struct StabilityProviderState {
    pub version: u8,
    pub stability_pool_state: Pubkey,
    pub owner: Pubkey,

    // State
    pub user_id: u64,
    pub deposited_stablecoin: u64,
    pub user_deposit_snapshot: DepositSnapshot,
    pub cumulative_gains_per_user: StabilityTokenMap,
    pub pending_gains_per_user: StabilityCollateralAmounts,
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize, Default)]
pub struct DepositSnapshot {
    pub sum: StabilityTokenMap,
    pub product: u128,
    pub scale: u64,
    pub epoch: u64,
    pub enabled: bool,
}

#[account(zero_copy)]
pub struct EpochToScaleToSumAccount {
    pub data: [u128; 1000],
}

impl Default for EpochToScaleToSumAccount {
    #[inline(never)]
    fn default() -> Self {
        Self { data: [0; 1000] }
    }
}

#[zero_copy]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CandidateRedemptionUser {
    pub status: u8,
    pub user_id: u64,
    pub user_metadata: Pubkey,
    pub debt: u64,
    pub collateral_ratio: u64,
    pub filler_metadata: Pubkey,
}

#[zero_copy]
#[derive(Debug, PartialEq, Eq)]
pub struct RedemptionOrder {
    pub id: u64,
    pub status: u8,
    pub base_rate: u16,
    pub last_reset: u64,
    pub redeemer_user_metadata: Pubkey,
    pub redeemer: Pubkey,
    pub requested_amount: u64,
    pub remaining_amount: u64,
    pub redemption_prices: TokenPrices,
    pub candidate_users: [CandidateRedemptionUser; 32],
}

#[account(zero_copy)]
pub struct RedemptionsQueue {
    pub orders: [RedemptionOrder; 15],
    pub next_index: u64,
}

#[zero_copy]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LiquidationEvent {
    // due to zero_copy we can't make this an enum
    // 0 - inactive
    // 1 - pending liquidation
    pub status: u8,
    pub user_positions: Pubkey,
    pub position_index: u64,
    pub liquidator: Pubkey,
    pub event_ts: u64,
    pub collateral_gain_to_liquidator: CollateralAmounts,
    pub collateral_gain_to_clearer: CollateralAmounts,
    pub collateral_gain_to_stability_pool: CollateralAmounts,
}

#[account(zero_copy)]
pub struct LiquidationsQueue {
    pub len: u64,
    pub events: [LiquidationEvent; 300],
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize, Default, StructArithmetic)]
pub struct TokenMap {
    pub sol: u128,
    pub eth: u128,
    pub btc: u128,
    pub srm: u128,
    pub ray: u128,
    pub ftt: u128,
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize, Default, StructArithmetic)]
pub struct StabilityTokenMap {
    pub sol: u128,
    pub eth: u128,
    pub btc: u128,
    pub srm: u128,
    pub ray: u128,
    pub ftt: u128,
    pub hbb: u128,
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize, Default)]
pub struct Price {
    // Pyth price, integer + exponent representation
    // decimal price would be
    // as integer: 6462236900000, exponent: 8
    // as float:   64622.36900000

    // value is the scaled integer
    // for example, 6462236900000 for btc
    pub value: u64,

    // exponent represents the number of decimals
    // for example, 8 for btc
    pub exp: u8,
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize, Default)]
pub struct TokenPrices {
    pub sol: Price,
    pub eth: Price,
    pub btc: Price,
    pub srm: Price,
    pub ray: Price,
    pub ftt: Price,
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize, Default, StructArithmetic)]
pub struct CollateralAmounts {
    pub sol: u64,
    pub eth: u64,
    pub btc: u64,
    pub srm: u64,
    pub ray: u64,
    pub ftt: u64,
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize, Default, StructArithmetic)]
pub struct StabilityCollateralAmounts {
    pub sol: u64,
    pub eth: u64,
    pub btc: u64,
    pub srm: u64,
    pub ray: u64,
    pub ftt: u64,
    pub hbb: u64,
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum CollateralToken {
    SOL = 0,
    ETH = 1,
    BTC = 2,
    SRM = 3,
    RAY = 4,
    FTT = 5,
}

#[derive(FromPrimitive, PartialEq, Eq, Clone, Copy)]
pub enum UserStatus {
    Inactive = 0,
    Active = 1,
    Liquidated = 2,
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum StabilityToken {
    SOL = 0,
    ETH = 1,
    BTC = 2,
    SRM = 3,
    RAY = 4,
    FTT = 5,
    HBB = 6,
}

impl StabilityToken {
    pub fn from(num: u8) -> StabilityToken {
        use StabilityToken::*;
        match num {
            0 => SOL,
            1 => ETH,
            2 => BTC,
            3 => SRM,
            4 => RAY,
            5 => FTT,
            6 => HBB,
            _ => unimplemented!(),
        }
    }
}

impl CollateralToken {
    pub fn from(num: u8) -> CollateralToken {
        use CollateralToken::*;
        match num {
            0 => SOL,
            1 => ETH,
            2 => BTC,
            3 => SRM,
            4 => RAY,
            5 => FTT,
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CollateralToken;

    #[test]
    fn test_token_convert() {
        let btc = CollateralToken::from(2);
        assert_eq!(btc, CollateralToken::BTC)
    }
}

// #[cfg(test)]
impl UserMetadata {
    pub fn to_state_string(&self) -> String {
        format!(
            "UserMetadata {{ user_id: {}, borrowed_stablecoin: {} deposited_collateral: {:?} }}",
            self.user_id, self.borrowed_stablecoin, self.deposited_collateral
        )
    }
}
