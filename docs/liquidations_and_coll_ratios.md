
# Borrowing and liquidation metrics

Users can deposit and borrow up to 110% of their collateral in most of the cases. However, there are situations when users can only borrow up to ~150% of their ratio. This is to keep the system solvent in case of a global decrease in collateral values.

## Definitions
- ICR = Individual Collateral Ratio
- MCR = Minimum Collateral Ratio
- TCR = Total Collateral Ratio
- SP = Stability Pool
- Recovery mode - A state of the system when TCR < 150%
- Normal mode - A state of the system when TCR > 150%
- Base rate 
    - a factor added to each borrowing/liquidation event
    - borrowing_fee = borrowing_amount * (BASE_RATE + 0.5%)
    - redemption_fee = redemption_amount * (BASE_RATE + 0.5%)
    - base_rate grows and decays with each borrowing and redemption event 

## Recovery mode 
- https://docs.liquity.org/faq/recovery-mode
- https://medium.com/liquity/liquity-releases-updated-whitepaper-e5e9fca8d8c7 
- The system blocks borrower transactions that would further decrease the TCR. New DEBT may only be issued by adjusting existing positions in a way that improves their collateral ratio, or by opening a new position with a collateral ratio>=150%. In general, if an existing position's adjustment reduces its collateral ratio, the transaction is only executed if the resulting TCR is above 150%.

In Recovery Mode, only allow:
- [x] Pure collateral top-up
- [x] Pure debt repayment
- [x] Collateral top-up with debt repayment
- [x] A debt increase combined with a collateral top-up which makes the ICR >= 150% and improves the ICR (and by extension improves the TCR).

In Normal Mode, ensure:
- [x] The new ICR is above MCR
- [x] The adjustment won't pull the TCR below CCR

In Recovery Mode, positions can be liquidated if their collateral ratio is below the TCR. In Normal Mode, only troves below 110% are subject to liquidation. If the system is in recovery mode (TCR < 150%), users with a collateral ratio below the TCR can be liquidated, but can only lose up 110% of their collateral.


| Condition | Liquidation Behavior |
| --- | ---|
| ICR <=100%     |                                Redistribute all debt and collateral (minus ETH gas compensation) to active positions. |
| 100% < ICR < MCR & SP LUSD > position debt       | LUSD in the Stability Pool equal to the position's debt is offset with the position's debt. The position's ETH collateral (minus ETH gas compensation) is shared between depositors. |
| 100% < ICR < MCR & SP LUSD < position debt |       The total Stability Pool LUSD is offset with an equal amount of debt from the position. A fraction of the position's collateral (equal to the ratio of its offset debt to its entire debt) is shared between depositors. The remaining debt and collateral (minus ETH gas compensation) is redistributed to active positions. |
| MCR <= ICR < 150% & SP LUSD >= position debt  |    The Stability Pool LUSD is offset with an equal amount of debt from the position. A fraction of ETH collateral with dollar value equal to 1.1 * debt is shared between depositors. Nothing is redistributed to other active positions. Since its ICR was > 1.1, the position has a collateral remainder, which is sent to the CollSurplusPool and is claimable by the borrower. The position is closed.|                         
| MCR <= ICR < 150% & SP LUSD < position debt |      Do nothing.
| ICR >= 150%   |                                 Do nothing.