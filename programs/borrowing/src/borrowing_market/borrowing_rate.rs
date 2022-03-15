use crate::utils::consts::{
    BORROWING_FEE_FLOOR, MAX_BORROWING_FEE, MAX_REDEMPTION_FEE, MINUTE_DECAY_FACTOR,
    REDEMPTION_FEE_FLOOR, SECONDS_PER_MINUTE,
};
use crate::BorrowingMarketState;
use decimal_wad::{
    common::{TryAdd, TryDiv, TryMul},
    decimal::Decimal,
    rate::Rate,
};

#[derive(Debug, Clone)]
pub struct BorrowSplit {
    // This includes the total amount of debt the user incurrs
    // amount_to_borriw - fees_to_pay is how much the user actually
    // gets in their wallet
    pub amount_to_borrow: u64,
    pub fees_to_pay: u64,
}

pub(crate) enum FeeEvent {
    Borrowing,
    Redemption { redeeming: u64, supply: u64 },
}

impl BorrowSplit {
    #[cfg(test)]
    pub fn from_amount(amount_to_borrow: u64, base_rate_bps: u16) -> Self {
        let borrowing_rate = calc_borrowing_fee(base_rate_bps);
        Self::split_fees(amount_to_borrow, borrowing_rate)
    }

    pub fn split_fees(requested_amount: u64, borrowing_rate: u16) -> BorrowSplit {
        let scaled_amount = Decimal::from(requested_amount);
        let borrowing_rate_scaled = Rate::from_bps(borrowing_rate);
        let scaled_fee = scaled_amount.try_mul(borrowing_rate_scaled).unwrap();
        // favour the protocol & stakers
        let fee = scaled_fee.try_ceil_u64().unwrap();

        BorrowSplit {
            amount_to_borrow: requested_amount.checked_add(fee).unwrap(),
            fees_to_pay: fee,
        }
    }
}

pub(crate) fn refresh_base_rate(
    market: &mut BorrowingMarketState,
    event: FeeEvent,
    now: u64,
) -> Result<(), crate::BorrowError> {
    let mut new_rate = decay_base_rate(market.base_rate_bps, market.last_fee_event, now);
    if let FeeEvent::Redemption { redeeming, supply } = event {
        new_rate = increase_base_rate(new_rate, supply, redeeming)?;
    };

    market.last_fee_event = u64::max(now, market.last_fee_event);
    market.base_rate_bps = new_rate;

    Ok(())
}

pub(crate) fn calc_redemption_fee(base_rate: u16) -> u16 {
    // between 0.5% and 100%
    u16::min(
        REDEMPTION_FEE_FLOOR.saturating_add(base_rate),
        MAX_REDEMPTION_FEE,
    )
}

pub(crate) fn calc_borrowing_fee(base_rate: u16) -> u16 {
    // between 0.5% and 5%
    u16::min(
        BORROWING_FEE_FLOOR.saturating_add(base_rate),
        MAX_BORROWING_FEE,
    )
}

pub(crate) fn decay_base_rate(base_rate: u16, last_fee_event: u64, now: u64) -> u16 {
    // Due to borrowing

    // Half-life of 12h. 12h = 720 min
    // (1/2) = d^720 => d = (1/2)^(1/720)

    // b(t) = b (t-1) * (1/2)^(minutes_passed/720)

    let old_base_rate = Rate::from_bps(base_rate);

    // cannot be negative
    let seconds_diff = now.checked_sub(last_fee_event).unwrap_or(0);

    let minutes_diff = seconds_diff / SECONDS_PER_MINUTE;
    let decay_factor = Rate::from_scaled_val(MINUTE_DECAY_FACTOR);
    let decay_factor = decay_factor.try_pow(minutes_diff).unwrap();
    let new_base_rate = old_base_rate.try_mul(decay_factor).unwrap();

    // println!(
    //     "Decaying base rate from {:?} to {:?} decay factor {:?} minutes {}",
    //     old_base_rate, new_base_rate, decay_factor, minutes_diff
    // );

    new_base_rate.to_bps().unwrap() as u16
}

pub(crate) fn increase_base_rate(
    old_base_rate: u16,
    total_usdh_supply: u64,
    total_usdh_redeemed: u64,
) -> Result<u16, crate::BorrowError> {
    // Due to redemptions

    // baseRate is decayed based on time passed since the last fee event
    // baseRate is incremented by an amount proportional to the fraction of the total LUSD supply that was redeemed
    // baseRate is incremented as
    // b(t) = b(t-1) + 0.5 * redeemed/total_supply
    // b(t) = b(t-1) + change
    // The redemption fee is given by (baseRate + 0.5%) * ETHdrawn

    if total_usdh_supply == 0 || total_usdh_redeemed == 0 {
        return Err(crate::BorrowError::ZeroAmountInvalid);
    }

    let base_rate = Rate::from_bps(old_base_rate);
    let change = {
        let fraction = if total_usdh_redeemed == total_usdh_supply {
            Rate::one()
        } else {
            let redeemed = Rate::from_scaled_val(total_usdh_redeemed);
            let supply = Rate::from_scaled_val(total_usdh_supply);
            redeemed.try_div(supply)?
        };
        fraction.try_div(2)?
    };

    let new_base_rate = base_rate.try_add(change)?;
    let new_base_rate = new_base_rate.to_bps()? as u16;

    let max_base_rate = 100 * 100; // 100% as bps
    let new_base_rate = u16::min(new_base_rate, max_base_rate);

    // #[cfg(test)]
    // println!(
    //     "Increasing base rate from {} to {} due to redeeming {} out of {}, as pct {:.3}%",
    //     old_base_rate,
    //     new_base_rate,
    //     total_usdh_redeemed,
    //     total_usdh_supply,
    //     (total_usdh_redeemed as f64 / total_usdh_supply as f64) * 100.0
    // );
    Ok(new_base_rate)
}
