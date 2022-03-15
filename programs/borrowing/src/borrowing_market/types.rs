use crate::{CollateralAmounts, LiquidationEvent};

#[derive(Debug)]
pub struct BorrowStablecoinEffects {
    pub amount_mint_to_user: u64,
    pub amount_mint_to_fees_vault: u64,
    pub amount_mint_to_treasury_vault: u64,
}

#[derive(Debug)]
pub struct DepositCollateralEffects {
    pub collateral_to_transfer_from_user: CollateralAmounts,
}

#[derive(Debug)]
pub struct RepayLoanEffects {
    pub amount_to_burn: u64,
    pub amount_to_transfer: u64,
}

#[derive(Debug)]
pub struct WithdrawCollateralEffects {
    pub collateral_to_transfer_to_user: CollateralAmounts,
    pub close_user_metadata: bool,
}

#[derive(Debug)]
pub struct LiquidationEffects {
    pub liquidation_event: LiquidationEvent,
    pub usd_to_burn_from_stability_pool: u64,
}

#[derive(Debug)]
pub struct ClearLiquidationGainsEffects {
    pub clearing_agent_gains: CollateralAmounts,
    pub stability_pool_gains: CollateralAmounts,
}

#[derive(Default, Debug)]
pub struct DepositAndBorrowEffects {
    pub amount_mint_to_user: u64,
    pub amount_mint_to_fees_vault: u64,
    pub amount_mint_to_treasury_vault: u64,
    pub collateral_to_transfer_from_user: CollateralAmounts,
}

impl From<DepositCollateralEffects> for DepositAndBorrowEffects {
    fn from(e: DepositCollateralEffects) -> Self {
        DepositAndBorrowEffects {
            collateral_to_transfer_from_user: e.collateral_to_transfer_from_user,
            ..Default::default()
        }
    }
}

impl From<BorrowStablecoinEffects> for DepositAndBorrowEffects {
    fn from(e: BorrowStablecoinEffects) -> Self {
        DepositAndBorrowEffects {
            amount_mint_to_user: e.amount_mint_to_user,
            amount_mint_to_fees_vault: e.amount_mint_to_fees_vault,
            amount_mint_to_treasury_vault: e.amount_mint_to_treasury_vault,
            ..Default::default()
        }
    }
}
