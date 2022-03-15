import * as set_up from '../src/set_up';
import { BorrowingGlobalAccounts, LiquidatorAccounts, StabilityPoolAccounts, StabilityProviderAccounts, StabilityProviderState } from '../src/set_up';
import * as anchor from '@project-serum/anchor';
import * as assert from "assert";

import { Keypair, PublicKey, Signer } from '@solana/web3.js';

import * as instructions_borrow from '../src/instructions_borrow';
import * as instructions_stability from '../src/instructions_stability';
import * as operations_staking from "./operations_staking";
import { displayBorrowingMarketState, displayStabilityPoolState, displayStabilityProviderState } from '../src/utils_display';
import { CollateralToken, numberToCollateralToken, StabilityToken } from './types';
import * as utils from "../src/utils";
import { decimalToU64, lamportsToColl, solAccountWithMinBalance } from "../src/utils";
import { getForcedSolBalanceInLamports, getTokenAccountBalance } from "./data_provider";
import { initialiseBorrowingMarkets } from "./operations_borrowing";

export async function initialiseStabilityPool(
    provider: anchor.Provider,
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingAccounts: set_up.BorrowingGlobalAccounts
) {
    const stabilityAccounts = await set_up.setUpStabilityPoolAccounts(
        provider,
        program,
        initialMarketOwner,
        borrowingAccounts
    );

    await instructions_stability
        .initializeStabilityPool(
            program,
            initialMarketOwner,
            borrowingAccounts,
            stabilityAccounts,
        );

    console.log('Initialized stability pool');

    return stabilityAccounts;
}

export async function newStabilityPoolUserWithPubkeys(
    provider: anchor.Provider,
    program: anchor.Program,
    stabilityVaults: PublicKey,
    epochToScaleToSum: PublicKey,
    stablecoinStabilityPoolVault: PublicKey,
    borrowingMarketState: PublicKey,
    stablecoinMint: PublicKey,
    stabilityPoolState: PublicKey,
    hbbMint: PublicKey,
    mintEth: PublicKey,
    mintBtc: PublicKey,
    mintSrm: PublicKey,
    mintRay: PublicKey,
    mintFtt: PublicKey,
    stablecoinBalance: number,
): Promise<StabilityProviderState> {

    const { keyPair: stabilityProvider } = await solAccountWithMinBalance(provider, 3);

    const stabilityProviderAccounts = await set_up.setUpStabilityProviderUserAccountsWithPubkeys(
        provider,
        [stabilityProvider],
        stabilityProvider.publicKey,
        program,
        hbbMint,
        stablecoinMint,
        mintEth,
        mintBtc,
        mintSrm,
        mintRay,
        mintFtt,
    );

    await instructions_stability.approveStability(
        program,
        stabilityProvider.publicKey,
        stabilityProviderAccounts.stabilityProviderState,
        stabilityPoolState,
        [stabilityProvider]
    );

    if (stablecoinBalance > 0) {
        await instructions_borrow.airdropStablecoin(
            program,
            provider.wallet.publicKey,
            borrowingMarketState,
            stabilityProviderAccounts.stablecoinAta,
            stablecoinMint,
            decimalToU64(stablecoinBalance),
        );
    }

    const usdhUserBalance = await getTokenAccountBalance(program, stabilityProviderAccounts.stablecoinAta);
    console.log("After airdrop usdh balance", stabilityProviderAccounts.stablecoinAta.toString(), usdhUserBalance);
    assert.strictEqual(usdhUserBalance, stablecoinBalance);

    const initialLamports = await getForcedSolBalanceInLamports(provider, stabilityProvider.publicKey);

    return {
        stabilityProvider,
        stabilityProviderAccounts,
        stabilityProviderInitialBalance: lamportsToColl(initialLamports, "SOL"),
    }
}

export async function newStabilityPoolUser(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingAccounts: set_up.BorrowingGlobalAccounts,
    stabilityPoolAccounts: set_up.StabilityPoolAccounts,
    stablecoinBalance: number,
): Promise<{ stabilityProvider: Keypair, stabilityProviderAccounts: StabilityProviderAccounts }> {

    return newStabilityPoolUserWithPubkeys(
        provider,
        program,
        stabilityPoolAccounts.stabilityVaults.publicKey,
        stabilityPoolAccounts.epochToScaleToSum,
        stabilityPoolAccounts.stablecoinStabilityPoolVault,
        borrowingAccounts.borrowingMarketState.publicKey,
        borrowingAccounts.stablecoinMint,
        borrowingAccounts.stabilityPoolState.publicKey,
        borrowingAccounts.hbbMint,
        borrowingAccounts.ethMint,
        borrowingAccounts.btcMint,
        borrowingAccounts.srmMint,
        borrowingAccounts.rayMint,
        borrowingAccounts.fttMint,
        stablecoinBalance,
    )
}

export async function newStabilityProviderWithPubkeys(
    provider: anchor.Provider,
    program: anchor.Program,
    stabilityVaults: PublicKey,
    epochToScaleToSum: PublicKey,
    stablecoinStabilityPoolVault: PublicKey,
    borrowingMarketState: PublicKey,
    stablecoinMint: PublicKey,
    stabilityPoolState: PublicKey,
    hbbMint: PublicKey,
    mintEth: PublicKey,
    mintBtc: PublicKey,
    mintSrm: PublicKey,
    mintRay: PublicKey,
    mintFtt: PublicKey,
    stablecoinToProvide: number,
): Promise<StabilityProviderState> {

    const { stabilityProvider, stabilityProviderAccounts, stabilityProviderInitialBalance } = await newStabilityPoolUserWithPubkeys(
        provider,
        program,
        stabilityVaults,
        epochToScaleToSum,
        stablecoinStabilityPoolVault,
        borrowingMarketState,
        stablecoinMint,
        stabilityPoolState,
        hbbMint,
        mintEth,
        mintBtc,
        mintSrm,
        mintRay,
        mintFtt,
        stablecoinToProvide,
    )

    if (stablecoinToProvide > 0) {
        await instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingMarketState,
            stabilityPoolState,
            stabilityVaults,
            epochToScaleToSum,
            stablecoinStabilityPoolVault,
            stabilityProviderAccounts.stablecoinAta,
            decimalToU64(stablecoinToProvide),
            [stabilityProvider]
        );
    }

    return {
        stabilityProvider,
        stabilityProviderAccounts,
        stabilityProviderInitialBalance
    }
}

export async function newStabilityProvider(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingAccounts: set_up.BorrowingGlobalAccounts,
    stabilityPoolAccounts: set_up.StabilityPoolAccounts,
    stablecoinToProvide: number,
): Promise<StabilityProviderState> {

    return newStabilityProviderWithPubkeys(
        provider,
        program,
        stabilityPoolAccounts.stabilityVaults.publicKey,
        stabilityPoolAccounts.epochToScaleToSum,
        stabilityPoolAccounts.stablecoinStabilityPoolVault,
        borrowingAccounts.borrowingMarketState.publicKey,
        borrowingAccounts.stablecoinMint,
        borrowingAccounts.stabilityPoolState.publicKey,
        borrowingAccounts.hbbMint,
        borrowingAccounts.ethMint,
        borrowingAccounts.btcMint,
        borrowingAccounts.srmMint,
        borrowingAccounts.rayMint,
        borrowingAccounts.fttMint,
        stablecoinToProvide,
    )
}

export async function newLiquidator(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingGlobalAccounts: set_up.BorrowingGlobalAccounts,
): Promise<{ liquidator: Keypair, liquidatorAccounts: LiquidatorAccounts, liquidatorInitialBalance: number }> {

    const { keyPair: liquidator, account: liquidatorAccount } = await utils.solAccountWithMinBalance(provider, 3);
    const liquidatorAccounts = await set_up.setUpLiquidatorAccounts(provider, liquidator, borrowingGlobalAccounts);

    return {
        liquidator,
        liquidatorAccounts,
        liquidatorInitialBalance: lamportsToColl(liquidatorAccount.lamports, "SOL"),
    }
}

export async function tryLiquidate(
    program: anchor.Program,
    liquidator: Keypair,
    borrowingGlobalAccounts: set_up.BorrowingGlobalAccounts,
    stabilityPoolGlobalAccounts: set_up.StabilityPoolAccounts,
    borrowerAccounts: set_up.BorrowingUserAccounts,
    liquidatorAccounts: set_up.LiquidatorAccounts,
    pythPrices: set_up.PythPrices,
    clear_gains: boolean = true
) {
    // Liquidate (at fake price)
    await instructions_borrow.tryLiquidate(
        program,
        liquidator.publicKey,
        borrowingGlobalAccounts.borrowingMarketState.publicKey,
        borrowingGlobalAccounts.stabilityPoolState.publicKey,
        borrowerAccounts.userMetadata.publicKey,
        stabilityPoolGlobalAccounts.epochToScaleToSum,
        stabilityPoolGlobalAccounts.stabilityVaults.publicKey,
        borrowingGlobalAccounts.borrowingVaults.publicKey,
        stabilityPoolGlobalAccounts.liquidationsQueue,
        borrowingGlobalAccounts.stablecoinMint,
        stabilityPoolGlobalAccounts.stablecoinStabilityPoolVault,
        pythPrices,
        [liquidator]
    );

    // Also, harvest liquidator gains
    if (clear_gains) {
        for (let i = 0; i < 6; i++) {
            await clearLiquidationGains(
                program,
                liquidator.publicKey,
                borrowingGlobalAccounts,
                stabilityPoolGlobalAccounts,
                liquidatorAccounts,
                [liquidator],
                numberToCollateralToken(i)
            )
        }
    }

    console.log("After liquidation");
    await displayStabilityPoolState(program, borrowingGlobalAccounts.stabilityPoolState.publicKey);
    await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
}

export async function clearLiquidationGains(
    program: anchor.Program,
    liquidator: PublicKey,
    borrowingAccounts: BorrowingGlobalAccounts,
    stabilityPoolAccounts: StabilityPoolAccounts,
    liquidatorAccounts: LiquidatorAccounts,
    signers: Array<Signer>,
    token: CollateralToken = "SOL") {

    let liquidationRewardsVault;
    let collateralVault;
    let liquidatorAta;

    switch (token) {
        case "SOL": {
            liquidationRewardsVault = stabilityPoolAccounts.liquidationRewardsVaultSol;
            collateralVault = borrowingAccounts.collateralVaultSol;
            liquidatorAta = liquidatorAccounts.solAta;
        } break;
        case "BTC": {
            liquidationRewardsVault = stabilityPoolAccounts.liquidationRewardsVaultBtc;
            collateralVault = borrowingAccounts.collateralVaultBtc;
            liquidatorAta = liquidatorAccounts.btcAta;
        } break;
        case "ETH": {
            liquidationRewardsVault = stabilityPoolAccounts.liquidationRewardsVaultEth;
            collateralVault = borrowingAccounts.collateralVaultEth;
            liquidatorAta = liquidatorAccounts.ethAta;
        } break;
        case "SRM": {
            liquidationRewardsVault = stabilityPoolAccounts.liquidationRewardsVaultSrm;
            collateralVault = borrowingAccounts.collateralVaultSrm;
            liquidatorAta = liquidatorAccounts.srmAta;
        } break;
        case "RAY": {
            liquidationRewardsVault = stabilityPoolAccounts.liquidationRewardsVaultRay;
            collateralVault = borrowingAccounts.collateralVaultRay;
            liquidatorAta = liquidatorAccounts.rayAta;
        } break;
        case "FTT": {
            liquidationRewardsVault = stabilityPoolAccounts.liquidationRewardsVaultFtt;
            collateralVault = borrowingAccounts.collateralVaultFtt;
            liquidatorAta = liquidatorAccounts.fttAta;
        } break;
    };

    await instructions_stability.clearLiquidationGains(
        program,
        liquidator,
        liquidatorAta,
        borrowingAccounts.borrowingMarketState.publicKey,
        borrowingAccounts.borrowingVaults.publicKey,
        borrowingAccounts.stabilityPoolState.publicKey,
        stabilityPoolAccounts.stabilityVaults.publicKey,
        stabilityPoolAccounts.liquidationsQueue,
        collateralVault,
        liquidationRewardsVault,
        signers,
        token,
    )
}

export async function harvestLiquidationGains(
    program: anchor.Program,
    stabilityProvider: Keypair,
    borrowingGlobalAccounts: set_up.BorrowingGlobalAccounts,
    stabilityPoolAccounts: set_up.StabilityPoolAccounts,
    stabilityProviderAccounts: set_up.StabilityProviderAccounts,
    harvestToken: StabilityToken = "SOL",
) {
    await instructions_stability.harvestLiquidationGains(
        program,
        stabilityProvider.publicKey,
        stabilityProviderAccounts.stabilityProviderState.publicKey,
        borrowingGlobalAccounts.borrowingMarketState.publicKey,
        borrowingGlobalAccounts.borrowingVaults.publicKey,
        borrowingGlobalAccounts.stabilityPoolState.publicKey,
        stabilityPoolAccounts.stabilityVaults.publicKey,
        stabilityPoolAccounts.epochToScaleToSum,
        stabilityPoolAccounts.liquidationsQueue,
        getLiquidationRewardsVaultForToken(stabilityPoolAccounts, harvestToken),
        getStabilityProviderAtaForToken(stabilityProvider.publicKey, stabilityProviderAccounts, harvestToken),
        borrowingGlobalAccounts.hbbMint,
        stabilityProviderAccounts.hbbAta,
        [stabilityProvider],
        harvestToken
    );

    console.log("After liquidation harvest");
    await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
    await displayStabilityPoolState(program, borrowingGlobalAccounts.stabilityPoolState.publicKey);
    await displayStabilityProviderState(program, stabilityProviderAccounts.stabilityProviderState.publicKey);
}

export function getLiquidationRewardsVaultForToken(
    stabilityPoolAccounts: set_up.StabilityPoolAccounts,
    token: StabilityToken
) {
    switch (token) {
        case "SOL": { return stabilityPoolAccounts.liquidationRewardsVaultSol }
        case "ETH": { return stabilityPoolAccounts.liquidationRewardsVaultEth }
        case "BTC": { return stabilityPoolAccounts.liquidationRewardsVaultBtc }
        case "SRM": { return stabilityPoolAccounts.liquidationRewardsVaultSrm }
        case "RAY": { return stabilityPoolAccounts.liquidationRewardsVaultRay }
        case "FTT": { return stabilityPoolAccounts.liquidationRewardsVaultFtt }
        default: { throw new Error(`Unsupported liquidation rewards vault token - ${token}`) }
    }
}

export function getStabilityProviderAtaForToken(
    stabilityProvider: PublicKey,
    stabilityProviderAccounts: set_up.StabilityProviderAccounts,
    token: StabilityToken
) {
    switch (token) {
        case "SOL": { return stabilityProvider }
        case "ETH": { return stabilityProviderAccounts.ethAta }
        case "BTC": { return stabilityProviderAccounts.btcAta }
        case "SRM": { return stabilityProviderAccounts.srmAta }
        case "RAY": { return stabilityProviderAccounts.rayAta }
        case "FTT": { return stabilityProviderAccounts.fttAta }
        default: { throw new Error(`Unsupported stability provider ATA token - ${token}`) }
    }
}

export type StabilityTestAccounts = {
    stabilityPoolAccounts: set_up.StabilityPoolAccounts,
    borrowingAccounts: set_up.BorrowingGlobalAccounts,
    stakingPoolAccounts: set_up.StakingPoolAccounts,
}

export async function createMarketAndStabilityPool(
    env: set_up.Env
): Promise<StabilityTestAccounts> {

    const { borrowingMarketAccounts, stakingPoolAccounts } = await operations_staking.initalizeMarketAndStakingPool(env)

    const stabilityAccounts = await initialiseStabilityPool(
        env.provider,
        env.program,
        env.initialMarketOwner,
        borrowingMarketAccounts,
    );

    return {
        stabilityPoolAccounts: stabilityAccounts,
        borrowingAccounts: borrowingMarketAccounts,
        stakingPoolAccounts
    }

}