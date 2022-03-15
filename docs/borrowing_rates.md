# Borrowing rates

## Types of fees
There are two times when fees are involved:
- at borrowing time
- at redemption time

At borrowing time, the fee is calculated based on the borrowed amount and is added on top of the debt of the user. For example, if a user borrows 100 USDH and the fee rate is 0.5%, then the following transfers happen:
- Hubble mints 100 USDH into borrower's wallet
- Hubble mints 0.5 USDH into the Stakers' pool
- The borrower's Hubble account records a debt of 100.5.

At redemption time, the fee is taken away from the redeemed collateral. Let's say a user wants to redeem 1000 USDH which is worth 1 SOL. Then the following things happen:
- Redeemer burns 1000 USDH
- Redeemer gets 0.995 SOL
- Stakers get 0.004 SOL
- (Bots) get 0.001 SOL (bots are needed for the redemption process)
- The user being redeemed against wipes out 1000 USDH from their debt, but lose 1 SOL from their collateral

## The fees dynamics
The fees should, 99% of the time, be fixed: 0.5%. However, due to demand and supply, the amount of USDH in circulation could be too much or too little, affecting the price peg. To counteract and rebalance these forces, a dynamic rate is created.

Borrowing indicates demand for USDH, therefore we respond to that by lowering the borrowing fee with every borrowing event.

Redemptions indicate too much supply of USDH (people sell it for USDH). So redemptions should increase the borrowing fee with every redemption event.

Also, the borrowing fee should be used to encourage borrowing when that's necessary, i.e. during Recovery mode. Therefore the borrowing fee is set to 0 during the Recovery periods.

The borrowing/redemption fee is based on a variable called BASE_RATE, which starts at 0. 

The fees imposed are:
- For borrowing: `borrowing_amount * (BASE_RATE + 0.5%)` (up to a max of 5%)
- For borrowing during recovery mode: `0`
- For redemption: `redemption_amount * (BASE_RATE + 0.5%)` (up to a max of 100%)
- For redemption during recovery mode: same as above, has no impact.

The formulas to update the fees are as follows:
- Borrowing events decrease the BASE_RATE - it halves every 12 hours:
    - `base_rate (t) = base_rate (t-1) * (1/2)^(minutes_passed/720)`
    - if mintes_passed == 720 (12 hours) then the base_rate decreases to half
- Redemption events increase the BASE_RATE - it increases by half of the amount redeemed
    - `base_rate (t) = base_rate (t-1) + 0.5 * usdh_redeemed/usdh_total_supply`

- test borrowing rate updates at the right points in execution path
    - [x] borrowing 
    - [ ] redemption

## Borrowing fee
- Borrowing indicates demand for USDH, therefore we respond to that by lowering the fee with every borrowing event.
- The fee decays with every borrowing event 
    - opening a debt position
    - taking on more debt
- Recovery mode: borrowing fee is set to 0 during recovery mode to increase borrowing
- Decay method: half every 12h

