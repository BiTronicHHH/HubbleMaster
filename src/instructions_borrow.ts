import * as anchor from '@project-serum/anchor';
import { Keypair, PublicKey, Signer, Transaction } from "@solana/web3.js";
import { TokenInstructions } from "@project-serum/serum";
import { CollateralToken, collateralTokenToNumber, StabilityToken, stabilityTokenToNumber } from '../tests/types';
import { BorrowingGlobalAccounts, LiquidatorAccounts, PythPrices, StabilityPoolAccounts, StabilityProviderAccounts } from './set_up';
import { getBorrowingMarketState, getStabilityVaults, getBorrowingVaults, getStakingPoolState, getGlobalConfig } from "../tests/data_provider";
import { mapAnchorError } from "./utils";
import { GlobalConfigOption } from "./config";

export async function initializeBorrowingMarket(
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingGlobalAccounts: BorrowingGlobalAccounts
) {
    let { borrowingMarketState, borrowingVaults, globalConfig } = borrowingGlobalAccounts;
    const tx = await program.rpc.initializeBorrowingMarket({
        accounts: {
            ...borrowingGlobalAccounts,
            initialMarketOwner,
            borrowingMarketState: borrowingMarketState.publicKey,
            borrowingVaults: borrowingVaults.publicKey,
            globalConfig: globalConfig.publicKey,
            redemptionsQueue: borrowingGlobalAccounts.redemptionsQueue,
            borrowingFeesVault: borrowingGlobalAccounts.borrowingFeesVault,
            burningVault: borrowingGlobalAccounts.burningVault,
            collateralVaultSol: borrowingGlobalAccounts.collateralVaultSol,
            collateralVaultSrm: borrowingGlobalAccounts.collateralVaultSrm,
            collateralVaultEth: borrowingGlobalAccounts.collateralVaultEth,
            collateralVaultBtc: borrowingGlobalAccounts.collateralVaultBtc,
            collateralVaultRay: borrowingGlobalAccounts.collateralVaultRay,
            collateralVaultFtt: borrowingGlobalAccounts.collateralVaultFtt,
            stablecoinMint: borrowingGlobalAccounts.stablecoinMint,
            hbbMint: borrowingGlobalAccounts.hbbMint,
            srmMint: borrowingGlobalAccounts.srmMint,
            ethMint: borrowingGlobalAccounts.ethMint,
            btcMint: borrowingGlobalAccounts.btcMint,
            rayMint: borrowingGlobalAccounts.rayMint,
            fttMint: borrowingGlobalAccounts.fttMint,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        },
        signers: [borrowingMarketState, borrowingVaults, globalConfig]
    });
    console.log('initializeBorrowingMarket done signature:', tx);
}

export async function updateGlobalConfig(program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    key: GlobalConfigOption,
    value: number) {
    return await program.rpc.updateGlobalConfig(new anchor.BN(key.valueOf()), new anchor.BN(value), {
        accounts: {
            initialMarketOwner,
            globalConfig: borrowingGlobalAccounts.globalConfig.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
        },
    });
}



export async function initializeTrove(
    program: anchor.Program,
    owner: PublicKey,
    userMetadata: Keypair,
    borrowingMarketState: PublicKey,
    stablecoinAta: PublicKey,
    signers: Array<Signer>) {

    const tx = await program.rpc.approveTrove({
        accounts: {
            owner,
            userMetadata: userMetadata.publicKey,
            borrowingMarketState,
            stablecoinAta,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [...signers, userMetadata]
    });
    console.log('initializeTrove done signature:', tx);
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
        accounts: getProvideStabilityAccounts(
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
        accounts: getWithdrawStabilityAccounts(owner,
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

export async function depositCollateral(
    program: anchor.Program,
    owner: PublicKey,
    userMetadata: PublicKey,
    collateralVaultTo: PublicKey,
    collateralFrom: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    amount: number,
    signers: Array<Signer>,
    token: CollateralToken = "SOL") {

    const tx = await mapAnchorError(program.rpc.depositCollateral(
        new anchor.BN(amount), new anchor.BN(collateralTokenToNumber(token)),
        {
            accounts: {
                owner,
                borrowingMarketState,
                borrowingVaults,
                userMetadata,
                collateralFrom,
                collateralTo: collateralVaultTo,
                systemProgram: anchor.web3.SystemProgram.programId,
                tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            },
            signers
        }
    ));

    console.log('depositCollateral done signature:', tx);
}

export async function borrowStablecoin(
    program: anchor.Program,
    owner: PublicKey,
    userMetadata: PublicKey,
    stablecoinMint: PublicKey,
    stablecoinBorrowingAssociatedAccount: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    stakingPoolState: PublicKey,
    borrowingFeesVault: PublicKey,
    treasuryVault: PublicKey,
    pythPrices: PythPrices,
    amount: number,
    signers: Array<Signer>) {
    const { stablecoinMintAuthority } = await getBorrowingMarketState(program, borrowingMarketState);

    const tx = await mapAnchorError(program.rpc.borrowStablecoin(
        new anchor.BN(amount),
        {
            accounts: utils.getBorrowStablecoinAccounts(
                owner,
                userMetadata,
                stablecoinMint,
                stablecoinMintAuthority,
                stablecoinBorrowingAssociatedAccount,
                borrowingMarketState,
                borrowingVaults,
                stakingPoolState,
                borrowingFeesVault,
                treasuryVault,
                pythPrices
            ),
            signers
        }
    ));

    console.log('borrowStablecoin done signature:', tx);
}

export async function depositAndBorrow(
    program: anchor.Program,
    owner: PublicKey,
    userMetadata: PublicKey,
    stablecoinMint: PublicKey,
    stablecoinBorrowingAssociatedAccount: PublicKey,
    collateralVaultTo: PublicKey,
    collateralFrom: PublicKey,
    borrowingMarketState: PublicKey,
    stakingPoolState: PublicKey,
    borrowingVaults: PublicKey,
    borrowingFeesVault: PublicKey,
    treasuryVault: PublicKey,
    pythPrices: PythPrices,
    depositAmount: number,
    depositAsset: CollateralToken = "SOL",
    borrowAmount: number,
    signers: Array<Signer>,
) {
    const { stablecoinMintAuthority } = await getBorrowingMarketState(program, borrowingMarketState);

    const tx = await mapAnchorError(program.rpc.depositCollateralAndBorrowStablecoin(
        new anchor.BN(depositAmount),
        new anchor.BN(collateralTokenToNumber(depositAsset)),
        new anchor.BN(borrowAmount),
        {
            accounts: utils.getDepositAndBorrowAccounts(
                owner,
                userMetadata,
                stablecoinMint,
                stablecoinMintAuthority,
                stablecoinBorrowingAssociatedAccount,
                borrowingMarketState,
                borrowingVaults,
                stakingPoolState,
                borrowingFeesVault,
                treasuryVault,
                collateralFrom,
                collateralVaultTo,
                pythPrices
            ),
            signers
        }
    ));

    console.log('depositAndBorrow done signature:', tx);
}

export async function repayLoan(
    program: anchor.Program,
    owner: PublicKey,
    userMetadata: PublicKey,
    stablecoinMint: PublicKey,
    stablecoinBorrowingAssociatedAccount: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    burningVault: PublicKey,
    pythPrices: PythPrices,
    amount: number,
    signers: Array<Signer>) {

    const { stablecoinMintAuthority } = await getBorrowingMarketState(program, borrowingMarketState);
    const { burningVaultAuthority } = await getBorrowingVaults(program, borrowingVaults);

    const tx = await mapAnchorError(program.rpc.repayLoan(new anchor.BN(amount), {
        accounts: utils.getRepayLoanAccounts(
            owner,
            userMetadata,
            borrowingMarketState,
            borrowingVaults,
            stablecoinMint,
            stablecoinMintAuthority,
            burningVault,
            burningVaultAuthority,
            stablecoinBorrowingAssociatedAccount,
        ),
        signers
    }));

    console.log('repayLoan done signature:', tx);
}

export async function withdrawCollateral(
    program: anchor.Program,
    owner: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    userMetadata: PublicKey,
    collateralFrom: PublicKey,
    collateralTo: PublicKey,
    pythPrices: PythPrices,
    amount: number,
    signers: Array<Signer>,
    token: CollateralToken = "SOL") {

    const { collateralVaultsAuthority } = await getBorrowingVaults(program, borrowingVaults);

    console.log("Withdrawing collateralFrom", collateralFrom.toString());
    console.log("Withdrawing collateralFromAuth", collateralVaultsAuthority.toString());

    const tx = await mapAnchorError(program.rpc.withdrawCollateral(
        new anchor.BN(amount),                  // amount of collateral
        new anchor.BN(collateralTokenToNumber(token)),    // collateral type SOL == 0
        {
            accounts: {
                owner,
                borrowingMarketState,
                borrowingVaults,
                userMetadata,
                collateralFrom,
                collateralFromAuthority: collateralVaultsAuthority,
                collateralTo,
                tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                pythSolPriceInfo: pythPrices.solPythPrice.publicKey,
                pythBtcPriceInfo: pythPrices.btcPythPrice.publicKey,
                pythEthPriceInfo: pythPrices.ethPythPrice.publicKey,
                pythSrmPriceInfo: pythPrices.srmPythPrice.publicKey,
                pythRayPriceInfo: pythPrices.rayPythPrice.publicKey,
                pythFttPriceInfo: pythPrices.fttPythPrice.publicKey,
            },
            signers
        }
    ));

    console.log('withdrawCollateral done signature:', tx);
}

export async function airdropStablecoin(
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingMarketState: PublicKey,
    stablecoinAta: PublicKey,
    stablecoinMint: PublicKey,
    amount: number,
) {

    const borrowingMarketStateAccount = await getBorrowingMarketState(program, borrowingMarketState);
    const stablecoinMintAuthority = borrowingMarketStateAccount.stablecoinMintAuthority;

    console.log("user", initialMarketOwner.toString());
    console.log("borrowingMarketState", borrowingMarketState.toString());
    console.log("stablecoinAta", stablecoinAta.toString());
    console.log("stablecoinMint", stablecoinMint.toString());
    console.log("stablecoinMintAuthority", stablecoinMintAuthority.toString());

    const tx = await program.rpc.airdropUsdh(
        new anchor.BN(amount), {
        accounts: {
            initialMarketOwner,
            borrowingMarketState,
            stablecoinAta,
            stablecoinMint,
            stablecoinMintAuthority,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
        },
        // signers
    });
    console.log('airdropStablecoin done signature:', tx);
}

export async function airdropHbb(
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingMarketState: PublicKey,
    userHbbAta: PublicKey,
    hbbMint: PublicKey,
    amount: number,
) {
    let borrowingMarketStateAccount =
        await program.account.borrowingMarketState.fetch(borrowingMarketState);


    // @ts-ignore
    const hbb_mint_authority = borrowingMarketStateAccount.hbbMintAuthority;
    const hbbMintAuthority =
        new PublicKey(hbb_mint_authority.toString());


    const tx = await program.rpc.airdropHbb(new anchor.BN(amount), {
        accounts: {
            initialMarketOwner,
            borrowingMarketState,
            userHbbAta,
            hbbMint,
            hbbMintAuthority,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
    });

    console.log('airdropHbb done signature:', tx);
}

export async function tryLiquidate(
    program: anchor.Program,
    liquidator: PublicKey,
    borrowingMarketState: PublicKey,
    stabilityPoolState: PublicKey,
    userMetadata: PublicKey,
    epochToScaleToSum: PublicKey,
    stabilityVaults: PublicKey,
    borrowingVaults: PublicKey,
    liquidationsQueue: PublicKey,
    stablecoinMint: PublicKey,
    stablecoinStabilityPoolVault: PublicKey,
    pythPrices: PythPrices,
    signers: Array<Signer>
) {
    const { stablecoinMintAuthority } = await getBorrowingMarketState(program, borrowingMarketState);

    const { stablecoinStabilityPoolVaultAuthority } = await getStabilityVaults(program, stabilityVaults);

    const tx = await mapAnchorError(program.rpc.tryLiquidate({
        accounts: utils.getTryLiquidateAccounts(
            liquidator,
            borrowingMarketState,
            stabilityPoolState,
            userMetadata,
            epochToScaleToSum,
            stabilityVaults,
            borrowingVaults,
            liquidationsQueue,
            stablecoinMint,
            stablecoinMintAuthority,
            stablecoinStabilityPoolVault,
            stablecoinStabilityPoolVaultAuthority,
            pythPrices
        ),
        signers
    }));
    console.log('tryLiquidate done signature:', tx);
}


export namespace utils {

    export function getRepayLoanAccounts(
        owner: PublicKey,
        userMetadata: PublicKey,
        borrowingMarketState: PublicKey,
        borrowingVaults: PublicKey,
        stablecoinMint: PublicKey,
        stablecoinMintAuthority: PublicKey,
        burningVault: PublicKey,
        burningVaultAuthority: PublicKey,
        stablecoinBorrowingAssociatedAccount: PublicKey,
    ): any {
        return {
            owner,
            userMetadata,
            borrowingMarketState,
            borrowingVaults,
            stablecoinMint,
            stablecoinMintAuthority,
            burningVault,
            burningVaultAuthority,
            stablecoinBorrowingAssociatedAccount,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
        };
    }

    export function getBorrowStablecoinAccounts(
        owner: PublicKey,
        userMetadata: PublicKey,
        stablecoinMint: PublicKey,
        stablecoinMintAuthority: PublicKey,
        stablecoinBorrowingAssociatedAccount: PublicKey,
        borrowingMarketState: PublicKey,
        borrowingVaults: PublicKey,
        stakingPoolState: PublicKey,
        borrowingFeesVault: PublicKey,
        treasuryVault: PublicKey,
        pythPrices: PythPrices): any {
        return {
            owner,
            borrowingMarketState,
            borrowingVaults,
            stakingPoolState,
            userMetadata,
            stablecoinMint,
            stablecoinMintAuthority,
            stablecoinBorrowingAssociatedAccount,
            borrowingFeesVault,
            treasuryVault,
            pythSolPriceInfo: pythPrices.solPythPrice.publicKey,
            pythBtcPriceInfo: pythPrices.btcPythPrice.publicKey,
            pythEthPriceInfo: pythPrices.ethPythPrice.publicKey,
            pythSrmPriceInfo: pythPrices.srmPythPrice.publicKey,
            pythRayPriceInfo: pythPrices.rayPythPrice.publicKey,
            pythFttPriceInfo: pythPrices.fttPythPrice.publicKey,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
        };
    }

    export function getDepositAndBorrowAccounts(
        owner: PublicKey,
        userMetadata: PublicKey,
        stablecoinMint: PublicKey,
        stablecoinMintAuthority: PublicKey,
        stablecoinBorrowingAssociatedAccount: PublicKey,
        borrowingMarketState: PublicKey,
        borrowingVaults: PublicKey,
        stakingPoolState: PublicKey,
        borrowingFeesVault: PublicKey,
        treasuryVault: PublicKey,
        collateralFrom: PublicKey,
        collateralTo: PublicKey,
        pythPrices: PythPrices,): any {
        return {
            owner,
            borrowingMarketState,
            borrowingVaults,
            stakingPoolState,
            userMetadata,
            stablecoinMint,
            stablecoinMintAuthority,
            collateralFrom,
            collateralTo,
            stablecoinBorrowingAssociatedAccount,
            borrowingFeesVault,
            treasuryVault,
            pythSolPriceInfo: pythPrices.solPythPrice.publicKey,
            pythBtcPriceInfo: pythPrices.btcPythPrice.publicKey,
            pythEthPriceInfo: pythPrices.ethPythPrice.publicKey,
            pythSrmPriceInfo: pythPrices.srmPythPrice.publicKey,
            pythRayPriceInfo: pythPrices.rayPythPrice.publicKey,
            pythFttPriceInfo: pythPrices.fttPythPrice.publicKey,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
        };
    }

    export function getTryLiquidateAccounts(
        liquidator: PublicKey,
        borrowingMarketState: PublicKey,
        stabilityPoolState: PublicKey,
        userMetadata: PublicKey,
        epochToScaleToSum: PublicKey,
        stabilityVaults: PublicKey,
        borrowingVaults: PublicKey,
        liquidationsQueue: PublicKey,
        stablecoinMint: PublicKey,
        stablecoinMintAuthority: PublicKey,
        stablecoinStabilityPoolVault: PublicKey,
        stablecoinStabilityPoolVaultAuthority: PublicKey,
        pythPrices: PythPrices
    ): any {
        return {
            liquidator,
            borrowingMarketState,
            stabilityPoolState,
            userMetadata,
            epochToScaleToSum,
            stabilityVaults,
            borrowingVaults,
            liquidationsQueue,
            stablecoinMint,
            stablecoinMintAuthority,
            stablecoinStabilityPoolVault,
            stablecoinStabilityPoolVaultAuthority,
            pythSolPriceInfo: pythPrices.solPythPrice.publicKey,
            pythBtcPriceInfo: pythPrices.btcPythPrice.publicKey,
            pythEthPriceInfo: pythPrices.ethPythPrice.publicKey,
            pythSrmPriceInfo: pythPrices.srmPythPrice.publicKey,
            pythRayPriceInfo: pythPrices.rayPythPrice.publicKey,
            pythFttPriceInfo: pythPrices.fttPythPrice.publicKey,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        };
    }
}