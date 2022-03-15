import * as set_up from '../src/set_up';
import * as global from '../src/global';
import * as anchor from '@project-serum/anchor';
import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction, TransactionSignature } from '@solana/web3.js';

import * as instructions_borrow from '../src/instructions_borrow';
import * as utils from '../src/utils';

import { displayBorrowingMarketState, displayData, displayTrove, displayUserBalances } from '../src/utils_display';
import { BorrowingGlobalAccounts, BorrowingUserAccounts, BorrowingUserState, StakingPoolAccounts, UserAtas } from '../src/set_up';
import * as operations_staking from './operations_staking';
import { sleep } from '@project-serum/common';
import { lamportsToColl, solAirdropMin } from "../src/utils";
import { CollateralToken } from './types';
import { getUserMetadata } from "./data_provider";

export async function setUpBorrowingUserAccounts(
    provider: anchor.Provider,
    program: anchor.Program,
    user: Keypair,
    globalAccounts: BorrowingGlobalAccounts
): Promise<BorrowingUserAccounts> {

    return set_up.setUpBorrowingUserAccounts(
        provider,
        user.publicKey,
        [user],
        user.publicKey,
        globalAccounts);
}

export async function setUpMarketWithUser(
    env: set_up.Env,
    prices: set_up.PythPrices,
    borrowAmount: number,
    depositAmount: number,
    asset: CollateralToken = "SOL") {

    let { borrowingMarketAccounts, stakingPoolAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

    const {
        borrower,
        borrowerAccounts,
        userId,
    } = await newBorrowingUser(env, borrowingMarketAccounts, map.from(depositAmount + 1.0, asset));

    if (depositAmount > 0) {
        await depositCollateral(
            env.provider,
            env.program,
            depositAmount,
            borrower,
            borrowerAccounts,
            borrowingMarketAccounts,
            asset
        );
    }

    if (borrowAmount > 0) {
        await borrow(
            env.provider,
            env.program,
            borrowAmount,
            borrower,
            borrowerAccounts,
            borrowingMarketAccounts,
            stakingPoolAccounts,
            prices
        );
    }

    let borrowingUserState = {
        userId,
        borrower: borrower,
        borrowerAccounts: borrowerAccounts,
        borrowerInitialBalance: lamportsToColl(depositAmount, asset),
        borrowerInitialDebt: 0,
    };

    return {
        borrowingUserState,
        borrowingAccounts: borrowingMarketAccounts,
        stakingAccounts: stakingPoolAccounts
    }
}

export async function setUpMarketWithEmptyUser(provider: anchor.Provider, program: anchor.Program, initialMarketOwner: PublicKey,) {
    const user = anchor.web3.Keypair.generate();
    await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(10, "SOL"));

    const borrowingGlobalAccounts = await set_up.setUpBorrowingGlobalAccounts(
        provider,
        initialMarketOwner,
        program);

    await instructions_borrow
        .initializeBorrowingMarket(
            program,
            initialMarketOwner,
            borrowingGlobalAccounts
        );

    const stakingAccounts = await operations_staking.initialiseStakingPool(
        provider,
        program,
        initialMarketOwner,
        borrowingGlobalAccounts,
        1500);

    console.log('Initialized market');

    await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

    console.log(`Created Global Accounts`);

    const userAccounts = await set_up.setUpBorrowingUserAccounts(
        provider,
        user.publicKey,
        [user],
        user.publicKey,
        borrowingGlobalAccounts);

    console.log(`Created User Accounts`);

    await instructions_borrow
        .initializeTrove(
            program,
            user.publicKey,
            userAccounts.userMetadata,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            userAccounts.stablecoinAta,
            [user]);

    return {
        user,
        userAccounts,
        borrowingGlobalAccounts
    }

}

export async function initialiseBorrowingMarkets(
    env: set_up.Env
): Promise<{ borrowingAccounts: BorrowingGlobalAccounts, stakingPoolAccounts: StakingPoolAccounts }> {
    const borrowingGlobalAccounts = await set_up.setUpBorrowingGlobalAccounts(
        env.provider,
        env.initialMarketOwner,
        env.program);

    await instructions_borrow
        .initializeBorrowingMarket(
            env.program,
            env.initialMarketOwner,
            borrowingGlobalAccounts
        );

    console.log('Initialized market');
    await displayBorrowingMarketState(env.program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

    const stakingPoolAccounts = await operations_staking.initialiseStakingPool(
        env.provider,
        env.program,
        env.initialMarketOwner,
        borrowingGlobalAccounts,
        1500);

    console.log(`Created Global Accounts`);
    return {
        stakingPoolAccounts,
        borrowingAccounts: borrowingGlobalAccounts
    }
}

export async function airdropToUser(
    env: set_up.Env,
    globalAccounts: BorrowingGlobalAccounts,
    borrowerAccounts: BorrowingUserState,
    minBalances: Map<CollateralToken, number>,
) {

    for (const [token, value] of minBalances.entries()) {
        if (token !== "SOL") {
            await mintToAta(env.provider, globalAccounts, borrowerAccounts.borrowerAccounts, token, utils.collToLamports(value, token));
        } else {
            await solAirdropMin(env.provider, borrowerAccounts.borrower.publicKey, value);
        }
    }

}

export async function newBorrowingUser(
    env: set_up.Env,
    globalAccounts: BorrowingGlobalAccounts,
    minBalances: Map<CollateralToken, number>,
): Promise<BorrowingUserState> {

    const solAccount = await utils.solAccountWithMinBalance(env.provider, minBalances.get("SOL") || 1);
    const borrower = solAccount.keyPair;

    const borrowerAccounts = await setUpBorrowingUserAccounts(env.provider, env.program, borrower, globalAccounts);
    for (const [token, value] of minBalances.entries()) {
        if (token !== "SOL") {
            await mintToAta(env.provider, globalAccounts, borrowerAccounts, token, utils.collToLamports(value, token));
        }
    }

    await instructions_borrow
        .initializeTrove(
            env.program,
            borrower.publicKey,
            borrowerAccounts.userMetadata,
            globalAccounts.borrowingMarketState.publicKey,
            borrowerAccounts.stablecoinAta,
            [borrower]);
    const userMetadata = await getUserMetadata(env.program, borrowerAccounts.userMetadata.publicKey);
    await displayTrove(env.program, borrowerAccounts);
    console.log(`Created user '${userMetadata.userId}' -> ${borrower.publicKey}`)
    await displayUserBalances(env.provider, borrower.publicKey, globalAccounts, borrowerAccounts);

    const initialLamports = (await env.provider.connection.getAccountInfo(borrower.publicKey))?.lamports;

    return {
        userId: userMetadata.userId,
        borrower,
        borrowerAccounts,
        borrowerInitialBalance: lamportsToColl(initialLamports, "SOL"),
        borrowerInitialDebt: 0,
    };
}

export async function depositAndBorrow(
    env: set_up.Env,
    borrowingAccounts: BorrowingGlobalAccounts,
    stakingAccounts: StakingPoolAccounts,
    borrowerAccounts: BorrowingUserState,
    prices: set_up.PythPrices,
    borrowAmount: number,
    depositAmount: number,
    depositCollateral: CollateralToken = "SOL"
): Promise<void> {
    await instructions_borrow
        .depositAndBorrow(
            env.program,
            borrowerAccounts.borrower.publicKey,
            borrowerAccounts.borrowerAccounts.userMetadata.publicKey,
            borrowingAccounts.stablecoinMint,
            borrowerAccounts.borrowerAccounts.stablecoinAta,
            borrowingAccounts.collateralVaultSol,
            borrowerAccounts.borrower.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stakingPoolState.publicKey,
            borrowingAccounts.borrowingVaults.publicKey,
            borrowingAccounts.borrowingFeesVault,
            stakingAccounts.treasuryVault,
            prices,
            utils.collToLamports(depositAmount, depositCollateral),
            depositCollateral,
            utils.decimalToU64(borrowAmount),
            [borrowerAccounts.borrower]);
}

export async function airdropFromWallet(
    provider: anchor.Provider,
    program: anchor.Program,
    amount: number,
    to: PublicKey
) {
    let adminSC: Uint8Array = Uint8Array.from([
        241, 101, 13, 165, 53, 150, 114, 216, 162, 246, 157, 94, 156, 209, 145, 37,
        186, 13, 219, 120, 66, 196, 128, 253, 177, 46, 0, 70, 68, 211, 238, 83, 155,
        17, 157, 105, 115, 161, 0, 60, 146, 250, 19, 171, 63, 222, 211, 135, 37, 102,
        222, 216, 142, 131, 67, 196, 185, 182, 202, 219, 55, 24, 135, 90
    ]);
    let adminPK = Keypair.fromSecretKey(adminSC);

    const tx = new Transaction().add(
        SystemProgram.transfer({
            fromPubkey: adminPK.publicKey,
            toPubkey: to,
            lamports: amount,
        })
    );

    const signature = await utils.send(provider, tx, adminPK.publicKey, [adminPK]);
    console.log("Airdrop Signature", signature);

}

export async function airdropSol(
    provider: anchor.Provider,
    program: anchor.Program,
    minBalance: number,
    account: PublicKey
) {
    if (global.env.cluster === "localnet") {
        let currentLamports = 0;
        const airdropBatchAmount = Math.max(minBalance, 200);
        do {
            let solAccount = await provider.connection.getAccountInfo(account);
            currentLamports = utils.lamportsToColl(solAccount?.lamports, "SOL");
            try {
                await provider.connection.requestAirdrop(account, utils.collToLamports(airdropBatchAmount, "SOL"));
            } catch (e) {
                console.log("Could not get airdrop");
            }
            await sleep(100);
            solAccount = await provider.connection.getAccountInfo(account);
        } while (currentLamports < minBalance);
    } else if (global.env.cluster === "devnet") {
        const airdropBatchAmount = 0.015;
        while (true) {
            try {
                await airdropFromWallet(
                    provider,
                    program,
                    airdropBatchAmount * LAMPORTS_PER_SOL,
                    account);
                break;
            } catch (e) {
                console.log("Could not get airdrop");
            }
            await sleep(1000);
        };
    }
}

export async function mintToAta(
    provider: anchor.Provider,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    userAtas: UserAtas,
    token: CollateralToken,
    amount: number,
): Promise<void> {

    let destination = null;
    let mint = null;

    switch (token) {
        case "SOL":
            throw new Error("Cannot mint to SOL ATA")
        case "BTC":
            destination = userAtas.btcAta;
            mint = borrowingGlobalAccounts.btcMint;
            break;
        case "ETH":
            destination = userAtas.ethAta;
            mint = borrowingGlobalAccounts.ethMint;
            break;
        case "SRM":
            destination = userAtas.srmAta;
            mint = borrowingGlobalAccounts.srmMint;
            break;
        case "RAY":
            destination = userAtas.rayAta;
            mint = borrowingGlobalAccounts.rayMint;
            break;
        case "FTT":
            destination = userAtas.fttAta;
            mint = borrowingGlobalAccounts.fttMint;
            break;
    }

    await utils.mintTo(
        provider,
        mint,
        destination,
        amount,
    )
}

export async function newBorrowingUserWithPubkeys(
    provider: anchor.Provider,
    program: anchor.Program,
    minBalance: number,
    borrowingMarketState: PublicKey,
    stablecoinMint: PublicKey,
    mintEth: PublicKey,
    mintBtc: PublicKey,
    mintSrm: PublicKey,
    mintRay: PublicKey,
    mintFtt: PublicKey,) {

    const borrower = anchor.web3.Keypair.generate();
    await airdropSol(provider, program, minBalance, borrower.publicKey);
    await sleep(4000);

    const borrowerAccounts = await set_up.setUpBorrowingUserAccountsWithPubkeys(
        provider,
        borrower.publicKey,
        [borrower],
        borrower.publicKey,
        stablecoinMint,
        mintEth,
        mintBtc,
        mintSrm,
        mintRay,
        mintFtt);

    await instructions_borrow
        .initializeTrove(
            program,
            borrower.publicKey,
            borrowerAccounts.userMetadata,
            borrowingMarketState,
            borrowerAccounts.stablecoinAta,
            [borrower]);

    console.log(`Created borrowing user -> ${borrower.publicKey}`)
    let solAccount = await provider.connection.getAccountInfo(borrower.publicKey);

    return { borrower, borrowerAccounts, borrowerInitialBalance: lamportsToColl(solAccount?.lamports, "SOL") || 0 };
}

export async function depositCollateral(
    provider: anchor.Provider,
    program: anchor.Program,
    depositAmount: number,
    user: Keypair,
    userAccounts: BorrowingUserAccounts,
    globalAccounts: BorrowingGlobalAccounts,
    token: CollateralToken = "SOL") {

    await depositCollateralWithPubkey(
        provider,
        program,
        depositAmount,
        user,
        userAccounts,
        globalAccounts.borrowingMarketState.publicKey,
        globalAccounts.borrowingVaults.publicKey,
        globalAccounts.collateralVaultSol,
        globalAccounts.collateralVaultEth,
        globalAccounts.collateralVaultBtc,
        globalAccounts.collateralVaultSrm,
        globalAccounts.collateralVaultRay,
        globalAccounts.collateralVaultFtt,
        globalAccounts.stablecoinMint,
        globalAccounts.ethMint,
        globalAccounts.btcMint,
        globalAccounts.srmMint,
        globalAccounts.rayMint,
        globalAccounts.fttMint,
        globalAccounts.hbbMint,
        token);

    await displayData(program, provider, userAccounts, globalAccounts, user);
}

export async function depositCollateralWithPubkey(
    provider: anchor.Provider,
    program: anchor.Program,
    depositAmount: number,
    user: Keypair,
    userAccounts: BorrowingUserAccounts,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    collateralVaultSol: PublicKey,
    collateralVaultEth: PublicKey,
    collateralVaultBtc: PublicKey,
    collateralVaultSrm: PublicKey,
    collateralVaultRay: PublicKey,
    collateralVaultFtt: PublicKey,
    stablecoinMint: PublicKey,
    ethMint: PublicKey,
    btcMint: PublicKey,
    srmMint: PublicKey,
    rayMint: PublicKey,
    fttMint: PublicKey,
    hbbMint: PublicKey,
    token: CollateralToken = "SOL") {

    console.log(`${user.publicKey} Depositing collateral ${token} ${depositAmount}`);

    let collateralVaultTo = collateralVaultSol;
    let collateralFrom = user.publicKey;

    switch (token) {
        case "ETH": { collateralVaultTo = collateralVaultEth; collateralFrom = userAccounts.ethAta; break; }
        case "BTC": { collateralVaultTo = collateralVaultBtc; collateralFrom = userAccounts.btcAta; break; }
        case "SRM": { collateralVaultTo = collateralVaultSrm; collateralFrom = userAccounts.srmAta; break; }
        case "RAY": { collateralVaultTo = collateralVaultRay; collateralFrom = userAccounts.rayAta; break; }
        case "FTT": { collateralVaultTo = collateralVaultFtt; collateralFrom = userAccounts.fttAta; break; }
    }

    await instructions_borrow
        .depositCollateral(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            collateralVaultTo,
            collateralFrom,
            borrowingMarketState,
            borrowingVaults,
            utils.collToLamports(depositAmount, token),
            [user],
            token);

}

export async function borrow(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowStablecoin: number,
    user: Keypair,
    userAccounts: BorrowingUserAccounts,
    globalAccounts: BorrowingGlobalAccounts,
    stakingAccounts: StakingPoolAccounts,
    pythPrices: set_up.PythPrices
) {

    await borrowWithPubkeys(
        program,
        user,
        userAccounts,
        globalAccounts.stablecoinMint,
        globalAccounts.borrowingMarketState.publicKey,
        globalAccounts.borrowingVaults.publicKey,
        globalAccounts.stakingPoolState.publicKey,
        globalAccounts.borrowingFeesVault,
        stakingAccounts.treasuryVault,
        pythPrices,
        borrowStablecoin,
    )
    await displayData(program, provider, userAccounts, globalAccounts, user);
}

export async function borrowWithPubkeys(
    program: anchor.Program,
    user: Keypair,
    userAccounts: BorrowingUserAccounts,
    stablecoinMint: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    stakingPoolState: PublicKey,
    borrowingFeesAccount: PublicKey,
    treasuryVault: PublicKey,
    pythPrices: set_up.PythPrices,
    borrowStablecoin: number,
) {

    console.log(`${user.publicKey} Borrowing USDH ${borrowStablecoin} ${utils.decimalToU64(borrowStablecoin)}`);

    await instructions_borrow
        .borrowStablecoin(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            stablecoinMint,
            userAccounts.stablecoinAta,
            borrowingMarketState,
            borrowingVaults,
            stakingPoolState,
            borrowingFeesAccount,
            treasuryVault,
            pythPrices,
            utils.decimalToU64(borrowStablecoin),
            [user]
        );
}

export async function repay(
    provider: anchor.Provider,
    program: anchor.Program,
    repayStablecoin: number,
    user: Keypair,
    userAccounts: BorrowingUserAccounts,
    globalAccounts: BorrowingGlobalAccounts,
    pythPrices: set_up.PythPrices
) {
    console.log(`Repaying ${repayStablecoin} xUSD -> user ${user.publicKey}`);
    await instructions_borrow
        .repayLoan(
            program,
            user.publicKey,
            userAccounts.userMetadata.publicKey,
            globalAccounts.stablecoinMint,
            userAccounts.stablecoinAta,
            globalAccounts.borrowingMarketState.publicKey,
            globalAccounts.borrowingVaults.publicKey,
            globalAccounts.burningVault,
            pythPrices,
            utils.decimalToU64(repayStablecoin),
            [user]);
    console.log(`Repaid ${repayStablecoin} xUSD -> user ${user.publicKey}`);
    await displayData(program, provider, userAccounts, globalAccounts, user);
}

export async function withdrawSolCollateral(
    provider: anchor.Provider,
    program: anchor.Program,
    depositSol: number,
    user: Keypair,
    userAccounts: BorrowingUserAccounts,
    globalAccounts: BorrowingGlobalAccounts,
    pythPrices: set_up.PythPrices
) {

    console.log(`Withdrawing collateral ${depositSol} SOL -> user ${user.publicKey}`);
    await instructions_borrow
        .withdrawCollateral(
            program,
            user.publicKey,
            globalAccounts.borrowingMarketState.publicKey,
            globalAccounts.borrowingVaults.publicKey,
            userAccounts.userMetadata.publicKey,
            globalAccounts.collateralVaultSol,
            user.publicKey,
            pythPrices,
            utils.collToLamports(depositSol, "SOL"),
            [user],
            "SOL");
    console.log(`Withdrew collateral ${depositSol} SOL -> user ${user.publicKey}`);
    await displayData(program, provider, userAccounts, globalAccounts, user);
}

export async function newLoanee(
    env: set_up.Env,
    globalAccounts: BorrowingGlobalAccounts,
    stakingAccounts: StakingPoolAccounts,
    pythPrices: set_up.PythPrices,
    borrowStablecoin: number,
    collateral: Map<CollateralToken, number>,
): Promise<BorrowingUserState> {

    const minBalances = new Map<CollateralToken, number>(collateral.entries());
    minBalances.set("SOL", (minBalances.get("SOL") || 0) + 1);
    const response = await newBorrowingUser(env, globalAccounts, minBalances);
    for (const [token, amount] of collateral.entries()) {
        if (amount > 0) {
            await depositCollateral(env.provider, env.program, amount, response.borrower, response.borrowerAccounts, globalAccounts, token);
        }
    }
    if (borrowStablecoin > 0) {
        await borrow(env.provider, env.program, borrowStablecoin, response.borrower, response.borrowerAccounts, globalAccounts, stakingAccounts, pythPrices);
    }
    return {
        ...response,
        borrowerInitialDebt: borrowStablecoin,
    }
}

export namespace map {
    export function from(amount: number, token: CollateralToken): Map<CollateralToken, number> {
        return new Map<CollateralToken, number>([
            [token, amount]
        ])
    }
}