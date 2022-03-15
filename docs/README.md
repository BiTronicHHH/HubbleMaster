Hubble architecture

## Overview
The protocol allows `borrowers` to deposit a portfolio of collateral (up to 6 tokens) and mint `USDH`, a stablecoin issued by Hubble. Minting is allowed up to 90.9% of the deposited collateral value (or a 110% collateral ratio - that way the stablecoin is always backed by collateral). They pay a one off interest fee of `0.5%` of the minted amount, denominated in USDH.

To keep the stablecoin pegged to the dollar, we allow for `face value redemption` of the USDH token, which trades on stable-swap plaftorms like Mercurial or Saber. That means, any `arbitrageur` can buy USDH from the market and redeem it for underlying collateral. For example, if you start with 100$ USDT, and USDH trades at 0.7, then you can make an instant ~42% profit, by buying 142 USDH and coming to Hubble and redeeming it for the face value of $142 worth of collateral from the platform (SOL, ETH, BTC, etc). In this case, there is also a `0.5% fee` taken by the protocol for the redemption transaction, denominated in collateral. Redemptions are implemented via redemption bots (covered below).

The reverse side of the peg, when USDH becomes more expensive, is to start with USDT, buy SOL from the market, deposit it on the protocol, mint USDH for the entire value and sell it for USDT at a premium (since USDH is more expensive, you get more USDT), then convert back to SOL and then USDT, having completed the loop with a higher USDT value. To be eligible for this opportunity, you need to start from an existing loan position that is slightly overcollateralized such that you can mint 100% of your newly deposited collateral (minus the `0.5%` minting fee).

The arbitrage on both ends is possible on Solana due to high txn speed, low gas costs and low slippage of the stableswap platforms. The `0.5%` fee for borrowing and redemption will keep the peg within 0.995 and 1.005. Arbitrages within this range are not profitable.

The fees collected by the protocol (borrowing and redemption) are distributed to `HBB` (our second native coin) token holders, which `stake` their coins in our protocol, proportionally to their share of the staking pool.

Finally, when the `borrowers` become undercollateralized (coll ratio drops below 110%, due to market prices changing), we are in danger that their debt is no longer fully backed by collateral (i.e. the USDH they issued, somewhere on the market, is backed by their collateral and can be redeemed from the protocol), and therefore we need to cover the debt somehow. We have two strategies to do so:
(1) First is the Stability pool, a vault where users (stability providers) can deposit USDH in anticipation of liquidation events; when a liquidation happens, we wipe out the debt and burn an equal amount from the stability pool, but distribute the entire amount of the collateral to the stability providers; because liquidation happens at 110%, they make a 10% profit.
(2) Second is the redistribution mechanism, where we take the bad debt and corresponding collateral and distribute it proportionally to other debt holders, increasing their net value, but decreasing their collateral ratio.

Our `liquidation bot` will trigger a liquidate instruction which will branch out on either of the options depending on the stability pool having enough USDH to cover the bad debt.

## Whitepapers & Documentation
- We are essentially rewriting `Liquity` with multi-collateral and with `yield` on collateral
- Our litepaper, which highlights everything at a high level is here  https://docsend.com/view/86nqc5tm29km96c9
- The `Liquity` whitepaper is here https://docsend.com/view/bwiczmy
- `Liquity` FAQ https://docs.liquity.org/
- `Liquity` solidity implementation https://github.com/liquity/dev/tree/main/packages/contracts/contracts

## Architecture

### Design 
The system is designed with a few core global `Accounts` keeping track of the amounts in every vault/pool as well as some `per user` accounts. The goal of the protocol is to accurately bookkeep all the interactions and transfer tokens when necessary.

Our design pattern is the following:
```rs
pub fn instruction(ctx: Context<Accounts>, input1: input_type1, ...) -> ProgramResult {
    // inputs from
    let account_state1 = &mut ctx.accounts.account_state1;
    let account_state2 = &mut ctx.accounts.account_state2;
    ...
    // update state & calculate side effects
    let side_effects = IntructionEffects {
        amount_to_transfer,
        amount_to_mint,
        amount_to_burn
    } = operations::update_state(
        account_state1,
        account_state2,
        input1,
        ...)?;

    // transfer based on side effects
    token_operations::transfer(amount_to_transfer, ctx)?;
    token_operations::mint(amount_to_mint, ctx)?;
    token_operations::burn(amount_to_burn, ctx)
}
```

As long as `update_state` performs the math correctly, the amounts to transfer should always reflect the internal state. We essentially need to validate that `operations::*` are correctly implemented and that `ctx` accounts are privileged for the tansfers.

### Components

We have 5 main components:
1. Borrowing operations (controls everything related to user debt)
    - create debt position
    - deposit collateral
    - borrow usdh
    - repay usdh
    - liquidations
    - redistributions
    - base rate growth (planned)
2. Staking operations (distributes protocol fees from borrowing and redemptions)
    - stake
    - unstake HBB
    - harvest gains
3. Stability operations (essentially it's a pool of USDH that will cover liquidations in exchange for collatera) 
    - provide usdh
    - withdraw usdh
    - clear liquidation gains (bot based)
    - harvest liquidation gains
4. Redemption mechanism (the way we maintain peg, if USDH is trading at discount, people can buy it cheaply and redeem it at face value in exchange of collateral. The collateral comes from the users with the lowest collateral ratio first, so we need to sort them. because sorting is expensive, we split it into multiple instructions - driven by bots) 
    - add redemption order
    - fill redemption order
    - clear redemption order
5. Delegation mechanism 
    - WIP, we will delegate idle collateral to earn yield on other platforms

There is an interplay between them as well - borrower operations triggers a `distribute fee` event for the staking operations, stability operations wipes out debt from the borrowing operations, redemption affects borrowing operations as well as staking operations.

### Algorithms used
- staking 
    - For staking, we are using `https://solmaz.io/2019/02/24/scalable-reward-changing/`.
    - See reference python implementation at `staking_algo.py`
- stability 
    - For the stability pool we are using `https://raw.githubusercontent.com/liquity/liquity/master/papers/Scalable_Reward_Distribution_with_Compounding_Stakes.pdf` because the stability pool can be fully depleted and can turn users' stakes to 0. Also we are taking inspiration from `https://github.com/liquity/dev/blob/main/packages/contracts/contracts/StabilityPool.sol`. There are a few tricks introduced, see the below implementation
    - Reference `liquity` implementation https://github.com/liquity/dev/blob/main/packages/contracts/contracts/StabilityPool.sol
    - see reference python implementation at `stability_pool_algo.py`


## The five components
1. Borrowing
2. Liquidations
3. Staking
4. Redemption
5. Yield generation

### Borrowing
Users can borrow up to 110% of their collateral ratio. An extra fee is minted to the stakers and recored to the users' debt position.   

The liquidations happen based on a set of conditions that influence at which collateral ratio the user will be liquidated. See https://docs.liquity.org/faq/recovery-mode.

## Redemptions
See redemption_design.md

## Liquidations & Redistribution
See liquidation_design.md

## Risks
- epoch_to_scale_to_sum -> number of bytes to support millions of liquidations & how to scale it indefinitely
- Redemption queue filling with bad orders - small orders 
- Redemption algo - deadlock (semaphore system)
- safe math - precision loss & overflow
- account validation
- CollateralAmount/TokenMap struct scaling (introduce padding)

reward {
    total_stake 
    rewad_amount
    u128 reward_per_stake += rewad_amount / total_stake;
}
reward_per_stake -> continously growing -> can overflow?
