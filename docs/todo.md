# Smart contracts

- [wip] Andrei: Swap to USDC
    - [x] Part 1: BTC - USDC
        - new borrower -> deposit BTC, ETH
        - new borrower -> usdc_ata
        - find price .. 
        - rust: 
            - let withrawn_amount = withdraw_collateral()
            - user.deposited_btc -= swapped_amount; 
            - global.deposited_btc -= swapped_amount;
        - ts: btc_ata === ... usdc_ata === .. 
        -> swap_ix -> BTC: 0, usdc_ata: += px_btx * btc_deposit
    - [x] Part 2: ETH/SRM.. - USDC
    - [wip] Part 3: SOL -> wSOL
    - [wip] Part 4: noop -> SOL - usdc 
    - [x] marius: open chat with panaiotis
- [wip] Elliot: Safety checks
    - [x] validate every single account input
    - [x] staking 
    - [wip] pyth
    - [ ] team run through all accounts together
- [wip] Vali: feature toggles
    - [ ] add unit & integration tests
    - [x] accessors
    - [ ] marius: to create full list of configs

- [ ] Marius: Add way more integration tests between SP & borrowing for liquidations
- [ ] Marius: Convert Liquity integration tests to Hubble 

- [draft] Delegate to earn
    - [ ] mSol & delegate to earn
    - [ ] Port
    - [ ] Apricot
    - [ ] Reach for partnerships
- [ ] redemption stakers / treasury, don't give stakers any collateral - give it to the bots, talk to the team
- [ ] Padding:
    - [ ] to the global states
    - [ ] to user states
    - [ ] to EpochToScaleToSum
- [ ] Deployment
    - Deployment scripts
    - Bots deployment (heartbeat)
- Bots
    - [ ] Redemption
    - [wip] Liquidation - 
- [ ] Logging

- [ ] marius: speed up tests (async parallel await for setup)
- [ ] marius: ui frontend need to derive everything from files 
- [ ] scale EpochToScaleToSum

## Nice to have:
- [ ] follow up: MIN_BORROW, MIN_REDEEM configurable
- [ ] scale redemptions_queue, liquidations_queue
- [ ] buy SRM to get lower fees 
- [ ] add chainlink & switchboard 
- [ ] switchboard (nft)
- [ ] Vali: check andrei anchor::events()
- [ ] https://github.com/mozilla/grcov

# UI
- [ ] decimal.js
- [ ] add "pending_rewards" to the UI to users
- [ ] add empty borrowed amount loans to dashboard
- [ ] fix the withdraw collateral display them all UI
- [ ] calculate APR/APY

# Done
- [x] Marius: recovery mode, ICR, REDISTRIBUTION
    - [x] adjust trove
    - [x] create one-off trove opening (this we can do)
    - [x] deposit += inactive
    - [x] borrow amount
    - [x] adjustment trove
    - [x] fix redistribution