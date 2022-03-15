Recovery Mode

# What is Recovery Mode? 
Recovery Mode kicks in when the TCR of the system falls below 150%.
During Recovery Mode, Troves with a collateral ratio below 150% can be liquidated. 
Moreover, the system blocks borrower transactions that would further decrease the TCR. New LUSD may only be issued by adjusting existing Troves in a way that improves their collateral ratio, or by opening a new Trove with a collateral ratio>=150%. In general, if an existing Trove's adjustment reduces its collateral ratio, the transaction is only executed if the resulting TCR is above 150%. 

# What is the Total Collateral Ratio?
The Total Collateral Ratio or TCR is the ratio of the Dollar value of the entire system collateral at the current ETH:USD price, to the entire system debt. In other words, it's the sum of the collateral of all Troves expressed in USD, divided by the debt of all Troves expressed in LUSD.

# What is the purpose of Recovery Mode? 
The goal of Recovery Mode is to incentivize borrowers to behave in ways that promptly raise the TCR back above 150%, and to incentivize LUSD holders to replenish the Stability Pool.
Economically, Recovery Mode is designed to encourage collateral top-ups and debt repayments, and also itself acts as a self-negating deterrent: the possibility of it occurring actually guides the system away from ever reaching it. Recovery Mode is not a desirable state for the system. 

# What are the fees during Recovery Mode?
While Recovery Mode has no impact on the redemption fee, the borrowing fee is set to 0% to maximally encourage borrowing (within the limits described ).

# How can I make my Trove safe in Recovery Mode?
By increasing your collateral ratio to 150% or greater, your Trove will be protected from liquidation. This can be done by adding collateral, repaying debt, or both.

# Can I be liquidated if my collateral ratio is below 150% in Recovery Mode? 
Yes, you can be liquidated below 150% if your Trove's collateral ratio is smaller than 150%. In order to avoid liquidation in Normal Mode and Recovery Mode, a user should keep their collateral ratio above 150%. 

# How much of a Troves collateral can be liquidated in Recovery Mode? 
In Recovery Mode, liquidation loss is capped at 110% of a Trove's collateral. Any remainder, i.e. the collateral above 110% (and below the TCR), can be reclaimed by the liquidated borrower using the standard web interface.
This means that a borrower will face the same liquidation “penalty” (10%) in Recovery Mode as in Normal Mode if their Trove gets liquidated.


# Liquidation conditions and amounts

## Normal Mode 
- if below MCR - SP then Redistribute 

## Recovery Mode
- ICR < 100% -> redistribute all
- 100% < ICR < MCR -> offset as much as possible, and redistribute the remainder
- ICR >= MCR && ICR < TCR && debt <= usd_in_sp -> offset everything, capped at 110% loss
- else: do nothing

## Decision Tree
```rs
if mode == RecoveryMode {
    if ICR < 100 {
        RedistributeAll
    } else {
        if ICR < MCR {
            StabilityPoolThenRedistribute
        } else {
            if ICR < TCR {
                if debt <= usd_in_sp {
                    StabilityPoolAll
                } else {
                    DoNothing
                }
            } else {
                DoNothing
            }
        }
    }
} else {
    if ICR < MCR {
        StabilityPoolThenRedistribute
    }
}
```

