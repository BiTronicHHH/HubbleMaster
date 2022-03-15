# Liquidations

One way to look at loans/debt is as collateral backing some amount of USDH. If the collateral goes below 1:1 with the debt, that means there is some USDH somewhere in the ecosystem not fully collateralized, risking a loss of trust in the stablecoin (since redeemers cannot realize an arbitrage gain). Our goal, with the liquidation mechanism, is to find an equivalent amount of USDH from somewhere and burn it (in exchange for the collateral) or to find some other collateral that can back it up.

Liquidations will never impose a higher than 10% liquidation penalty. If someone is liquidated at 150% ratio (due to Recovery mode), they still keep 40% of their collateral.

That's why there are 2 solutions for liquidations: 
1. Liquidations via the stability pool and 
    - Liquidations via the stability pool (SP) are quite simple: SP providers will stake USDH ahead of time and when a liqudation happens, their USDH is burned and they receive the collateral instead. As we are liquidating at 110%, they will make a 10% gain.
2. liquidations via the `redistribution` of the debt to other debt owners (that are better collateralized).
    - Liquidations via redistribution happen by distributing the debt and the collateral to all the other debt positions. Since we redistribute at 110%, everyone will gain a little bit more collateral than debt, but their collateral ratio will drop slightly. Since people that receive redistribution gains are not at risk of liquidation (otherwise they would have been liquidated already) and have a higher collateral ratio than 110%, and as they receive some debt backed by 110% collateral, the new collateral ratio is higher than 110% and therefore the USDH out there in the ecosystem is safely backed by more collateral than before.
    - Please have a look at the `redistribution_algo.xlsx` to see how we do it

## Recovery mode: https://docs.liquity.org/faq/recovery-mode

## 1. Liquidations via the stability pool

Since we need to do everything in O(1), we need to keep track of the events. Normally, this would be done using the classic `rewards_per_token` index, but the problem here is that the Stability Pool could potentially `deplete` and your stake could become 0. In that case, you are no longer entitled to subsequent rewards. 

We are using the Liquity's paper:
- https://raw.githubusercontent.com/liquity/liquity/master/papers/Scalable_Reward_Distribution_with_Compounding_Stakes.pdf
- https://github.com/liquity/dev/blob/main/packages/contracts/contracts/StabilityPool.sol

We use a `hack` to keep track of a hashmap. Please see the `epoch_to_scale_to_sum.rs` file.

### Technical implementation:

A bot monitors for positions that might be undercollateralized and triggers a `try_liquidate` instruction. The instructions receives the oracles and the loan in question and checks if a liquidation is possible. If that's the case, it does two things:

1. Updates the debt position with 0 debt and `remaining` collateral (what's left after 110% collateral is taken away)
2. If there is USDH in the Stability Pool, it uses as much as possible from the SP and burns it and redistributes the collateral to liquidation rewards pools.
3. If there is not enough in the SP to cover the USDH, the remaining USDH and it's corresponding collateral is distributed to the other debt holders.

### Challenges

Due to the facts that:
- We keep a `collateral_vault` for each token in the Borrowing component
- We keep a `liquidations_vault` for each token in the Stability pool component
- The solana account limit and compute buget sizes

Then we cannot execute the `try_liquidate` instruction atomically:
- we need to move collateral from the collateral_vault_$(token) to the liquidation_reward_vault_$(token). 
- Given that we have more than 1 token and that the collateral_vault_$(token) is a PDA, we need to pass 3 accounts for each collateral transfer: collateral_vault, collateral_vault_authority and liquidation_rewards_vault for each token.
- Multiply that by at least 6 tokens
- Add oracle prices accounts
- and we run out of accounts for the transaction. 

Because of this, we delay the token transfers to another instruction.

### Solution

Due to lack of space in the accounts inputs, we cannot transfer all the collateral at once, i.e. from sol, eth, btc, coll vaults to the liquidation reward vaults & liquidator ata.

So, instead, we just update the state, and put a LiquidationEvent in a queue to be processed in a subsequent transaction. Bots will be incentivised to do so, and block any other action to be done until
that event is processed and there are no more events to be processed.

The state is correct, the `StabilityPoolState` is correctly updated with the latest debt levels, but if a `StabilityPoolProvider` wants to claim their gained collateral, they can't because it hasn't been moved from `collateral_vault` to `liquidations_rewards_vault`.

As such, harvesting events is blocked (because it involves withdrawing from the liquidations_rewards_vault), but not withdrawing and depositing, because they only updating state and removing from stability_pool_vault. 

To the rewards are moved from collateral vaults to liqudiation vaults, we use bots. Bots will be rewarded for executing permissionlessly the intruction `clear_liquidation_gains`.

There are two types of bots:
- the `liquidator` bot, which triggers the first `try_liquidate` transaction
- the `clearer` bot which triggers the `clear_liquidations_gains` transaction

They are both rewarded for doing this. The liquidator bot needs to trigger `clear` for themselves too, since it involves geting a % of the liquidated collateral into their own vaults. They have 5 seconds to do so. If they don't do it, the clearing bot will be entitled to claim them. This is done to ensure that the liquidations queue is cleared. 

## 2. Liquidations via redistribution

If there is nothing in the stability pool, then we need to find another way to back the undercollateralized USDH by some collateral. We do it by redistributing it among other debt holders.

See the example in the spreadsheet. We do it using `pending_reward_per_token` and the `apply_pending_rewards` function.

For example, if we need to distribute a debt of `100 USDH` which is backed only by `105` worth of SOL, then we distribute 100 USDH among all debt holders and 105 collateral to all. The amount they get is based on their debt percentage of the entire pool. For example

```
                user#1      user#2      user#3      user#4      user#5
SOL (in usd)    +100        +300        +60         +70         +70
BTC (in usd)    +100        +100        +60         +60         +40
Total Coll      +200        +400        +120        +130        +110
Debt (USDH)     -100        -100        -100        -100        -100
Net Value       +100        +300        +20         +30         +10
Coll. Ratio     200%        400%        120%        130%        110%
```

Let's say we liquidate user 5, the remaining users look like:
```
                user#1      user#2      user#3      user#4  
SOL (in usd)    +100        +300        +60         +70      
BTC (in usd)    +100        +100        +60         +60      
Total Coll      +200        +400        +120        +130     
Debt (USDH)     -100        -100        -100        -100     
Net Value       +100        +300        +20         +30  
Pool Pct        25%         25%         25%         25%  
Coll. Ratio     200%        400%        120%        130%
```

Then everyone gets 25% of the debt and 25% of the collateral:
```
                user#1      user#2      user#3      user#4  
SOL (in usd)    +117.5      +317.5      +77.5       +87.5      
BTC (in usd)    +115        +115        +75         +75      
Total Coll      +232.5      +432.5      +152.5      +162.5     
Debt (USDH)     -125        -125        -125        -125     
Net Value       +107.5      +307.5      +27.5       +37.5  
Coll. Ratio     186%        346%        122%        130%  
```

You can see that:
- Net value increased
- Collateral ratio decreased for some, incrased for others


## Epoch To Scale To Sum

- How it's used
StabilityPool {
    usdh_deposited: u64 -> 1000 usdh
    p: 00000000012,
    p: 00000000012 * 10000000,
    s: s+1,
    e: 1,
    s: +10
}

Epoch{
    epoch: u64,
    [Scale {
        scale: u64,
        sum: Tokenmap { 
            sol: u64,
             eth, ... 
        } 
    }]
} 

Liquidation (usdh: 600, coll_sol: 660)

sp.usdh_deposited -= liqui.usdh
sp.coll_gained += liqui.coll_sol
sp.p *= (1 - sp.usdh_deposited/liqui.usdh)
sp.p = 70%

StabilityProvider {
    user_id: 0,
    deposited: 500,
    p: 1.0000,
    e: 0,
    s: 1,
}

StabilityProvider {
    user_id: 0,
    deposited: 500,
    p: 1.0000
    e: 0,
    s: 3,
}

StabilityProvider {
    user_id: 0,
    deposited: 500,
    p: 0.7000
    e: 0,
}

- How to scale
1. Possibilty
EpochToScaleToSum {
    Vec<Vec<TokenMap>>
}

- epoch.get(userSnapshotEpoch, userSnapshotScale)
- epoch.get(userSnapshotEpoch, userSnapshotScale + 1)
- epoch.get(currentEpoch, currentScale)
- epoch.set(currentEpoch, currentScale)
- epoch.set(currentEpoch, currentScale + 1)
- epoch.set(currentEpoch + 1, 0)

EpochToScaleToSum {
    epoch: u64,
    scale: u64,
    sum: TokenMap {
        sol: u64,
        srm: u64
    }
}

EpochToScaleToSum {
    epoch: u64,
    scale: u64,
    sum: TokenMap {
        sol: u64,
        srm: u64,
        ..,
        ..,
        ..,
        ..
    }
}
2. Change encoding