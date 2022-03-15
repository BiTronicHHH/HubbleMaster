import * as set_up from '../src/set_up';
import * as utils from '../src/utils';
import * as anchor from '@project-serum/anchor';
import * as assert from "assert";

import { PublicKey } from '@solana/web3.js';

import * as instructions_staking from '../src/instructions_staking';
import * as instructions_borrowing from '../src/instructions_borrow';
import * as operations_borrowing from './operations_borrowing';

import { sleep } from '@project-serum/common';
import { CollateralToken } from "./types";


type StakingTestAccounts = {
    stakingPoolAccounts: set_up.StakingPoolAccounts,
    borrowingMarketAccounts: set_up.BorrowingGlobalAccounts,
}

export async function initialiseStakingPool(
    provider: anchor.Provider,
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingAccounts: set_up.BorrowingGlobalAccounts,
    treasuryFeeRate: number
) {
    const stakingAccounts = await set_up.setUpStakingPoolAccounts(
        provider,
        initialMarketOwner,
        program,
        borrowingAccounts
    );

    await instructions_staking.initializeStakingPool(
        program,
        initialMarketOwner,
        borrowingAccounts.borrowingMarketState.publicKey,
        borrowingAccounts.stakingPoolState,
        stakingAccounts.stakingVault,
        stakingAccounts.treasuryVault,
        treasuryFeeRate
    );

    console.log('Initialized staking pool');

    return stakingAccounts;
}

export async function initalizeMarketAndStakingPool(
    env: set_up.Env
): Promise<StakingTestAccounts> {

    const borrowingAccounts = await operations_borrowing.initialiseBorrowingMarkets(env);

    return {
        stakingPoolAccounts: borrowingAccounts.stakingPoolAccounts,
        borrowingMarketAccounts: borrowingAccounts.borrowingAccounts
    }
}

export async function newStakingPoolUser(
    provider: anchor.Provider,
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingAccounts: set_up.BorrowingGlobalAccounts,
    stakingPoolAccounts: set_up.StakingPoolAccounts,
    hbbToStake: number,
) {
    return newStakingPoolUserWithPubkeys(
        provider,
        program,
        initialMarketOwner,
        hbbToStake,
        borrowingAccounts.borrowingMarketState.publicKey,
        stakingPoolAccounts.stakingVault,
        borrowingAccounts.stakingPoolState.publicKey,
        borrowingAccounts.hbbMint,
        borrowingAccounts.stablecoinMint
    )
}

export async function newStakingPoolUserWithPubkeys(
    provider: anchor.Provider,
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    hbbToStake: number,
    borrowingMarketState: PublicKey,
    stakingVault: PublicKey,
    stakingPoolState: PublicKey,
    hbbMint: PublicKey,
    stablecoinMint: PublicKey
) {
    const user = anchor.web3.Keypair.generate();

    const minSolBalance = 5;
    const airdropBatchAmount = 5;

    let solAccount = await provider.connection.getAccountInfo(user.publicKey);
    while (utils.lamportsToColl(solAccount?.lamports, "SOL") < minSolBalance) {
        try {
            await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(airdropBatchAmount, "SOL"));
        } catch (e) { }
        await sleep(500)
        solAccount = await provider.connection.getAccountInfo(user.publicKey);
    }

    const userStakingPoolAccounts = await set_up.setUpStakingPoolUserAccountsWithPubkeys(
        provider,
        [user],
        user.publicKey,
        hbbMint,
        stablecoinMint);

    await instructions_staking.approveStakingPool(
        program,
        user.publicKey,
        userStakingPoolAccounts.userStakingState,
        stakingPoolState,
        [user]);

    await instructions_borrowing.airdropHbb(
        program,
        initialMarketOwner,
        borrowingMarketState,
        userStakingPoolAccounts.userHbbAta,
        hbbMint,
        hbbToStake);

    let hbbUserBalance = await provider.connection.getTokenAccountBalance(userStakingPoolAccounts.userHbbAta);
    console.log("After airdrop hbb balance", userStakingPoolAccounts.userHbbAta.toString(), hbbUserBalance.value.uiAmountString);
    console.log("HBB balance", JSON.stringify(hbbUserBalance.value));
    assert.strictEqual(Number.parseInt(hbbUserBalance.value.amount), hbbToStake);

    await instructions_staking.stake(
        program,
        user.publicKey,
        userStakingPoolAccounts.userStakingState.publicKey,
        borrowingMarketState,
        stakingPoolState,
        stakingVault,
        userStakingPoolAccounts.userHbbAta,
        [user],
        hbbToStake);

    return { user, userStakingPoolAccounts }
}

export async function triggerFees(
    env: set_up.Env,
    borrowingAccounts: set_up.BorrowingGlobalAccounts,
    stakingAccounts: set_up.StakingPoolAccounts,
    depositSol: number,
    borrowStablecoin: number,
    pythPrices: set_up.PythPrices
) {
    const { borrower, borrowerAccounts } = await operations_borrowing.newBorrowingUser(env, borrowingAccounts, new Map<CollateralToken, number>([
        ["SOL", depositSol + 1]
    ]));

    await operations_borrowing.depositCollateral(env.provider, env.program, depositSol, borrower, borrowerAccounts, borrowingAccounts);
    await sleep(1000);

    // borrow stable
    await operations_borrowing.borrow(env.provider, env.program, borrowStablecoin, borrower, borrowerAccounts, borrowingAccounts, stakingAccounts, pythPrices);

}