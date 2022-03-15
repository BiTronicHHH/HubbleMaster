import * as anchor from '@project-serum/anchor';
import { Keypair, PublicKey, Signer } from "@solana/web3.js";
import { TokenInstructions } from "@project-serum/serum";
import { getBorrowingMarketState, getBorrowingVaults, getStabilityVaults } from "../tests/data_provider";
import { mapAnchorError } from "./utils";
import { BorrowingGlobalAccounts, StabilityPoolAccounts } from './set_up';
import { CollateralToken, collateralTokenToNumber, StabilityToken, stabilityTokenToNumber } from '../tests/types';


export async function initializeStabilityPool(
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    stabilityAccounts: StabilityPoolAccounts
) {
    const { stabilityPoolState } = borrowingGlobalAccounts;
    const { stabilityVaults } = stabilityAccounts;
    const tx = await program.rpc.stabilityInitialize({
        accounts: utils.initializeStabilityPoolAccounts(initialMarketOwner, borrowingGlobalAccounts, stabilityAccounts),
        signers: [stabilityPoolState, stabilityVaults]
    });
    console.log('initializeStabilityPool done signature:', tx);
}
export async function approveStability(
    program: anchor.Program,
    owner: PublicKey,
    stabilityProviderState: Keypair,
    stabilityPoolState: PublicKey,
    signers: Array<Signer>) {
    const tx = await mapAnchorError(program.rpc.stabilityApprove({
        accounts: {
            owner,
            stabilityProviderState: stabilityProviderState.publicKey,
            stabilityPoolState,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [...signers, stabilityProviderState]
    }));
    console.log('approveStability done signature:', tx);
}

export async function provideStability(
    program: anchor.Program,
    owner: PublicKey,
    stabilityProviderState: PublicKey,
    borrowingMarketState: PublicKey,
    stabilityPoolState: PublicKey,
    stabilityVaults: PublicKey,
    epochToScaleToSum: PublicKey,
    stablecoinStabilityPoolVault: PublicKey,
    stablecoinAta: PublicKey,
    amount: number,
    signers: Array<Signer>) {

    const tx = await mapAnchorError(program.rpc.stabilityProvide(
        new anchor.BN(amount), {
        accounts: utils.getProvideStabilityAccounts(
            owner,
            stabilityProviderState,
            borrowingMarketState,
            stabilityPoolState,
            stabilityVaults,
            epochToScaleToSum,
            stablecoinStabilityPoolVault,
            stablecoinAta
        ),
        signers
    }));
    console.log('provideStability done signature:', tx);
}

export async function withdrawStability(
    program: anchor.Program,
    owner: PublicKey,
    stabilityProviderState: PublicKey,
    borrowingMarketState: PublicKey,
    stabilityPoolState: PublicKey,
    stabilityVaults: PublicKey,
    epochToScaleToSum: PublicKey,
    stablecoinStabilityPoolVault: PublicKey,
    stablecoinAta: PublicKey,
    amount: number,
    signers: Array<Signer>) {

    const { stablecoinStabilityPoolVaultAuthority } = await getStabilityVaults(program, stabilityVaults);

    const tx = await mapAnchorError(program.rpc.stabilityWithdraw(
        new anchor.BN(amount), {
        accounts: utils.getWithdrawStabilityAccounts(owner,
            stabilityProviderState,
            borrowingMarketState,
            stabilityPoolState,
            stabilityVaults,
            epochToScaleToSum,
            stablecoinStabilityPoolVault,
            stablecoinStabilityPoolVaultAuthority,
            stablecoinAta
        ),
        signers
    }));
    console.log('withdrawStability done signature:', tx);
}


export async function clearLiquidationGains(
    program: anchor.Program,
    clearingAgent: PublicKey,
    clearingAgentAta: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    stabilityPoolState: PublicKey,
    stabilityVaults: PublicKey,
    liquidationsQueue: PublicKey,
    collateralVault: PublicKey,
    liquidationRewardsVault: PublicKey,
    signers: Array<Signer>,
    token: CollateralToken = "SOL",
) {
    const { collateralVaultsAuthority } = await getBorrowingVaults(program, borrowingVaults);

    const tx = await mapAnchorError(program.rpc.clearLiquidationGains(
        new anchor.BN(collateralTokenToNumber(token)), {
        accounts: utils.getClearLiquidationGainsAccounts(
            clearingAgent,
            clearingAgentAta,
            borrowingMarketState,
            borrowingVaults,
            stabilityPoolState,
            stabilityVaults,
            liquidationsQueue,
            collateralVault,
            collateralVaultsAuthority,
            liquidationRewardsVault,
        ),
        signers
    }));
    console.log('clearLiquidationGains done signature:', tx);
}

export async function harvestLiquidationGains(
    program: anchor.Program,
    owner: PublicKey,
    stabilityProviderState: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    stabilityPoolState: PublicKey,
    stabilityVaults: PublicKey,
    epochToScaleToSum: PublicKey,
    liquidationsQueue: PublicKey,
    liquidationRewardsVault: PublicKey,
    liquidationRewardsTo: PublicKey,
    hbbMint: PublicKey,
    hbbAta: PublicKey,
    signers: Array<Signer>,
    harvestToken: StabilityToken = "SOL",
) {
    const stabilityVaultsAccount = await getStabilityVaults(program, stabilityVaults);
    const liquidationRewardsVaultAuthority = stabilityVaultsAccount.liquidationRewardsVaultAuthority;

    const borrowingMarketStateAccount = await getBorrowingMarketState(program, borrowingMarketState);
    const hbbMintAuthority = borrowingMarketStateAccount.hbbMintAuthority;

    const tx = await mapAnchorError(program.rpc.harvestLiquidationGains(
        new anchor.BN(stabilityTokenToNumber(harvestToken)), {
        accounts: utils.getHarvestLiquidationGainsAccounts(owner,
            stabilityProviderState,
            borrowingMarketState,
            borrowingVaults,
            stabilityPoolState,
            stabilityVaults,
            epochToScaleToSum,
            liquidationsQueue,
            liquidationRewardsVault,
            liquidationRewardsVaultAuthority,
            liquidationRewardsTo,
            hbbMint,
            hbbMintAuthority,
            hbbAta,
        ),
        signers
    }));
    console.log('harvestLiquidationGains done signature:', tx);
}

export namespace utils {

    export function initializeStabilityPoolAccounts(
        initialMarketOwner: PublicKey,
        borrowingGlobalAccounts: BorrowingGlobalAccounts,
        stabilityAccounts: StabilityPoolAccounts): any {

        const { borrowingMarketState, stabilityPoolState } = borrowingGlobalAccounts;
        const { stabilityVaults } = stabilityAccounts;
        return {
            initialMarketOwner,
            borrowingMarketState: borrowingMarketState.publicKey,
            stabilityPoolState: stabilityPoolState.publicKey,
            stabilityVaults: stabilityVaults.publicKey,
            epochToScaleToSum: stabilityAccounts.epochToScaleToSum,
            liquidationsQueue: stabilityAccounts.liquidationsQueue,
            liquidationRewardsVaultSol: stabilityAccounts.liquidationRewardsVaultSol,
            liquidationRewardsVaultSrm: stabilityAccounts.liquidationRewardsVaultSrm,
            liquidationRewardsVaultEth: stabilityAccounts.liquidationRewardsVaultEth,
            liquidationRewardsVaultBtc: stabilityAccounts.liquidationRewardsVaultBtc,
            liquidationRewardsVaultRay: stabilityAccounts.liquidationRewardsVaultRay,
            liquidationRewardsVaultFtt: stabilityAccounts.liquidationRewardsVaultFtt,
            stablecoinStabilityPoolVault: stabilityAccounts.stablecoinStabilityPoolVault,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        };
    }


    export function getProvideStabilityAccounts(
        owner: PublicKey,
        stabilityProviderState: PublicKey,
        borrowingMarketState: PublicKey,
        stabilityPoolState: PublicKey,
        stabilityVaults: PublicKey,
        epochToScaleToSum: PublicKey,
        stablecoinStabilityPoolVault: PublicKey,
        stablecoinAta: PublicKey
    ): any {
        return {
            owner,
            stabilityProviderState,
            borrowingMarketState,
            stabilityPoolState,
            stabilityVaults,
            epochToScaleToSum,
            stablecoinStabilityPoolVault,
            stablecoinAta,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        };
    }

    export function getWithdrawStabilityAccounts(
        owner: PublicKey,
        stabilityProviderState: PublicKey,
        borrowingMarketState: PublicKey,
        stabilityPoolState: PublicKey,
        stabilityVaults: PublicKey,
        epochToScaleToSum: PublicKey,
        stablecoinStabilityPoolVault: PublicKey,
        stablecoinStabilityPoolVaultAuthority: PublicKey,
        stablecoinAta: PublicKey
    ): any {

        return {
            owner,
            stabilityProviderState,
            borrowingMarketState,
            stabilityPoolState,
            stabilityVaults,
            epochToScaleToSum,
            stablecoinStabilityPoolVault,
            stablecoinStabilityPoolVaultAuthority,
            stablecoinAta,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        };
    }

    export function getClearLiquidationGainsAccounts(
        clearingAgent: PublicKey,
        clearingAgentAta: PublicKey,
        borrowingMarketState: PublicKey,
        borrowingVaults: PublicKey,
        stabilityPoolState: PublicKey,
        stabilityVaults: PublicKey,
        liquidationsQueue: PublicKey,
        collateralVault: PublicKey,
        collateralVaultsAuthority: PublicKey,
        liquidationRewardsVault: PublicKey,
    ): any {
        return {
            clearingAgent,
            clearingAgentAta,
            borrowingMarketState,
            borrowingVaults,
            stabilityPoolState,
            stabilityVaults,
            liquidationsQueue,
            collateralVault,
            collateralVaultsAuthority,
            liquidationRewardsVault,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        }
    }

    export function getHarvestLiquidationGainsAccounts(
        owner: PublicKey,
        stabilityProviderState: PublicKey,
        borrowingMarketState: PublicKey,
        borrowingVaults: PublicKey,
        stabilityPoolState: PublicKey,
        stabilityVaults: PublicKey,
        epochToScaleToSum: PublicKey,
        liquidationsQueue: PublicKey,
        liquidationRewardsVault: PublicKey,
        liquidationRewardsVaultAuthority: PublicKey,
        liquidationRewardsTo: PublicKey,
        hbbMint: PublicKey,
        hbbMintAuthority: PublicKey,
        hbbAta: PublicKey,
    ): any {
        return {
            owner,
            stabilityProviderState,
            borrowingMarketState,
            borrowingVaults,
            stabilityPoolState,
            stabilityVaults,
            epochToScaleToSum,
            liquidationsQueue,
            liquidationRewardsVault,
            liquidationRewardsVaultAuthority,
            liquidationRewardsTo,
            hbbMint,
            hbbMintAuthority,
            hbbAta,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        };
    }

}