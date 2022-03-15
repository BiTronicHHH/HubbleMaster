# Redemption

The redemption mechanism was designed to keep the stablecoin from dropping below the peg. When USDH is trading below peg, users can buy it cheaply on the market and redeem it at face value.

- Redeeming at face value means that for every 1 USDH you can get $1 worth of SOL (or other collateral).
- Example:
    - USDH is trading at 0.75 to USDC
    - SOL/USDC price is 100.0
    - Arbitrageur can
        1. Start with 1 SOL
        2. Swap 1 SOL to 100 USDC
        3. Buy 100 / 0.75 = 133.33 USDH
        4. Redeem 133.33 USDH from the Redemptions Page
        5. Get back 1.33 SOL (we started from 1 sol so we made a 0.33 gain)
- Buying USDH to redeem it at face value adds buy pressure and brings the price back up

## How can USDH drop below peg
- USDH should not really drop below peg because it will trade on a stablecoin exchange such as mercurial.fi or saber.so
- however, selling pressure (selling USDH to buy USDC) can lead to price drops
- in such cases users can do two things
    - borrowers can buy it cheaply in the market and pay back their debt
    - arbitrageurs can take advantage of the redemption mechanism to make arb gains

## How does the redemption mechanism work
- The entire point of our stablecoin is that it's overcollateralized
- Users don't need to trust us, as soon as it loses peg, they can (claim/burn) swap it for the underlying collateral and make gains
- The collateral needs to come from somewhere:
    - we don't have a treasury for these things
    - so the next best thing is to take it from the users that are most risky (have highest loan to value ratio)
    - (!) we can only redeem the amount of debt they have (not the entire collateral), and therefore they will not incur a net loss
- The redemption fee:
    - Each time there is a redemption, the redeemer will pay 0.5% + (Base Rate) of the collateral to the HBB holders 
- The base rate:
    - Each time there is a redemption, the Base Rate will increase to discourage further borrowing (since borrowing will make users dump more USDH on the market)

- An example:
    - Assuming the price of SOL is 100 dollars, user starts with 1 SOL balance
    - Assuming USDH is trading at discount (50%) USDH:USDC - 0.5
    - User sells 1 SOL to USDC and then buys USDH -> 200 USDH
    - Comes to Hubble and redeems 200 USDh 
        -> redeem as if 1USDH == 1USDC 
        -> user gets back 200 USD worth of collateral
        -> that is 2 SOL
    - User started with 1 SOL -> risk free arbitrage


## Redemption example

Let's start with this situation:
```
                user#1      user#2      user#3      user#4      user#5
SOL (in usd)    +100        +300        +60         +70         +115
BTC (in usd)    +100        +100        +60         +60          0
Total Coll      +200        +400        +120        +130        +115
Debt (USDH)     -100        -100        -100        -100        -100
Coll. Ratio     200%        400%        120%        130%        115%
Net Value       +100        +200        +20         +30         +15
```
- Notice:
    - Total amount that can be redeemed is 500 USDH (that's how much it has been minted and that's the max that exists in the market)
    - The users in increasing order of collateral ratio are: 5, 3, 4, 1, 2 (5 having the smallest)

### Partial redemption (single collateral)
So if USDH is trading at discount and someone buys it cheaply from Mercurial, they can redeem and make a gain. For example, if we want to redeem 20 USD we will partially redeem from user #5 since his total debt covers the amount of USDH we want to redeem. After redemption his position looks like this:
```
                Before: user#5      After: user#5
SOL (in usd)    +115                 95
BTC (in usd)    +0                   0
Total Coll      +115                 95
Debt (USDH)     -100                -80
Coll. Ratio     115%                 118.75%
Net Value        +15                 +15
```
Notice:
- The collateral ratio has improved!
- As you can see, the net value is the same - the user needs to pay back less debt, but get back less collateral, and overall it gets the same net value (the user loses exposure to SOL upside, but also is protected from SOL downside)
- if the SOL/USD price is 100, then the redeemer has received 0.195 SOL and HBB stakers have received 0.005 SOL.
- the profitability of this scenario depends at which price the redeember has bought the USDH.
- The redeember has earned back 99.5% of 20 USD (in SOL) and 0.5% of SOL is sent back to the HBB holders
- As you can tell, redeemers will not make a profit if the peg is only 0.995 since 0.5% is lost to the HBB stakers
- However, that doesn't mean that the stablecoin cannot go above 0.995, because borrowers (people who already have debt) can buy it and pay back their debt at 0 cost, driving the price up.


### Full redemption (multi coin)
- Let's say the user wants to redeem 150 USDH
- In this case, user #5 cannot fully cover the amount, so we need to redeem user #5 fully and user #3 partially. We will take:
    - 100 usd worth of SOL from user 1 and wipe out their debt entirely
    - 50 usd worth of SOL & BTC from user 2 and wipe out half of their debt
```
                Before: user#3  Before: user#5  After: user#3  After: user#5
SOL (in usd)    +60             +115            +35             +15
BTC (in usd)    +60             +0              +35             +0
Total Coll      +120            +115            +70             +15
Debt (USDH)     -100            -100            -50             -0
Coll. Ratio     120%            115%            140%            inf
Net Value       +20             +15             +20             +15
```
- The redeemer has redeemed 150 USD
    - user 5 was fully redeemed
    - user 3 was partially - 50% redeemed
- The net values of user 3 and 5 are the same and collateral ratios are much improved
- The redeemer has received 0.995 * 150 USD (in sol and btc)
- The HBB stakers have received 0.005 * 150 USD (in sol and btc)
- if SOL/USD == 100 (means user3 had 0.6 sol & user5 had 1.15 sol)
- if BTC/USD == 1000 (means user3 had 0.006 btc)
- then the redeemer received:
    - (100 + 25) in sol x 0.995 = 125 * 0.995 = 124.375 = 1.24375 sol
    - 25 in btc x 0.995 = 25 * 0.998 = 24.95 / 1000 = 0.02495 btc
- the HBB stakers received
    - 125 * 0.005 / 100  = 0.00625 SOL
    - 25 * 0.005 / 1000  = 0.000125 BTC
- 150 USDH was burned and that much debt wiped out from our state (books)

## Implementation details
The algorithm states:
- Sort the users in increasing order of collateral ratio
- Keep collecting collateral from the lowest up to the highest until the redeemed amount has been filled

There are a few challenges with that:
- So let's say we have N users with M tokens each and debt each
- We need to fill in a redemption order of `K` USDH
- We need to start adding up USDH from the users with the lowest collateral ratio until the redemption is filled
    - 0.5% of the collateral is sent to the HBB stakers
    - 99.5% is sent to the redeemer
    - USDH is transferred to the burning vault and burned
    - user positions need to be updated
- Because it is expensive to sort a vector of N users with M tokens each, it becomes NxM Log(N), we decided to split it using a `merge` sort approach.

## The merge sort algorithm

1. A redemption order is placed on the `redemptions_queue` with the requested amount and the snapshot of the prices at that time.
```rs
RedemptionOrder {
    order_id: 0,
    token_prices: {
        btc: 12.0,
        btc: 12.0,
        btc: 12.0,
    },
    requested_amount: 100,
    redeemed_amount: 0,
    last_collection_time_ts: 0,
    candidate_users: vec![CandidateUser::default();32]
}
```
Each redemption order has a max of 32 candidates users that can fit into the array. Due to the fact that we need to verify everything on chain, we need to keep track of that. 

2. A `filler` bot will submit `x` users at a time to the `fill_redemption_order` instruction. The program receives those users, compares them with the existing users and merge them. 
    - For example, if the `candidate_users` list is empty and the filler submits 
    - `[user1, user2, user3]` then the `candidate_users` list becomes the given users sorted ascending order by Collateral Ratio. If the user submits duplicates, we filter that out. 
    - If the list has `[user1, user2, user3, .., .., ..]` and another bot submits `[user1, user4, user5]`, then the list becomes a union of the two but merged by collateral ratio (the program always verifies).
    - We keep track about which bot (filler) has submitted which user. When we claim the collateral (during the `clear` instruction), that bot will e incentivised.
    - Fillers keep doing this repeatedly and the program keeps merging them and remembering which bots submitted which users. The bots that submit the best `lowest` users will get rewarded, so this creates an incentive to act safely. We will have our own bot to ensure that someone will try to outcompete us.
3. The `fillers` have 5 seconds to submit candidates after which the `clearer` bot starts popping candidates off the queue.
3. The job of the `clearer` is to start popping users off the queue. This essentially means, it starts with the lowest CR user, moves their collateral to the `Redeemer` account, wipes out the redeemed debt and burns USDH from the burning vault.
    - Once the clearer starts popping off, it needs to keep going until it finishes the queue (or the redeemed amount is filled)
    - The clearer is incentivised to do so and the instruction is permissionless, so anyone can write this bot
4. Since we can only fit a fixed amount of users in each Solana transaction and since our queue size is fixed, Steps 2 and 3 repeat until the requested redemption amount is filled.
4. At the end of the process the redeemer will convert all of their collected collateral into one single coin, via an instruction we will implement, to be able to realise the arbitrage.

