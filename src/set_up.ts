import * as anchor from "@project-serum/anchor";
import * as serumCmn from "@project-serum/common";

import { PythUtils } from "./pyth";
import { Connection, Keypair, PublicKey, Signer, Transaction } from "@solana/web3.js";

import * as utils from "./utils";
import * as global from "./global";

export const seeds_version = 7;
export const collateral_sol_seed = `collateral${seeds_version}`;
export const trove_seed = `trove${seeds_version}`;

export type BorrowingGlobalAccounts = {
    stablecoinMint: PublicKey;
    hbbMint: PublicKey;
    burningVault: PublicKey;
    borrowingFeesVault: PublicKey;
    borrowingMarketState: Keypair;
    borrowingVaults: Keypair;
    globalConfig: Keypair;
    stabilityPoolState: Keypair;
    stakingPoolState: Keypair;
    redemptionsQueue: PublicKey;
    collateralVaultSol: PublicKey;
    collateralVaultEth: PublicKey;
    collateralVaultBtc: PublicKey;
    collateralVaultSrm: PublicKey;
    collateralVaultRay: PublicKey;
    collateralVaultFtt: PublicKey;
    ethMint: PublicKey;
    btcMint: PublicKey;
    srmMint: PublicKey;
    rayMint: PublicKey;
    fttMint: PublicKey;
};

export type BorrowingUserState = {
    userId: number
    borrower: Keypair,
    borrowerAccounts: BorrowingUserAccounts,
    borrowerInitialBalance: number
    borrowerInitialDebt: number
}

export type BorrowingUserAccounts = UserAtas & {
    userMetadata: Keypair;
};

export type UserAtas = {
    stablecoinAta: PublicKey;
    ethAta: PublicKey;
    btcAta: PublicKey;
    srmAta: PublicKey;
    rayAta: PublicKey;
    fttAta: PublicKey;
};

export type StabilityPoolAccounts = {
    liquidationRewardsVaultSol: PublicKey,
    liquidationRewardsVaultEth: PublicKey,
    liquidationRewardsVaultSrm: PublicKey,
    liquidationRewardsVaultBtc: PublicKey,
    liquidationRewardsVaultRay: PublicKey,
    liquidationRewardsVaultFtt: PublicKey,
    stablecoinStabilityPoolVault: PublicKey;
    epochToScaleToSum: PublicKey;
    liquidationsQueue: PublicKey;
    stabilityVaults: Keypair
};

export type StabilityProviderState = {
    stabilityProvider: Keypair,
    stabilityProviderAccounts: StabilityProviderAccounts,
    stabilityProviderInitialBalance: number
}

export type StabilityProviderAccounts = UserAtas & {
    stabilityProviderState: Keypair
    hbbAta: PublicKey,
    solCollateralLiquidationsRewardsPending: PublicKey,
    hbbEmissionRewardsPending: PublicKey,
}

export type LiquidatorAccounts = {
    solAta: PublicKey,
    srmAta: PublicKey,
    ethAta: PublicKey,
    btcAta: PublicKey,
    rayAta: PublicKey,
    fttAta: PublicKey,
}

export type RewardsGlobalAccounts = {
    globalStakingState: PublicKey;
    rewardsPot: PublicKey;
    stakingVault: PublicKey;
    rewardsMint: PublicKey;
};

export type RewardsUserAccounts = {
    userStakingState: PublicKey;
    stakeRewardCoinAta: PublicKey;
    rewardCoinPendingPot: PublicKey;
};

export type StakingPoolAccounts = {
    stakingVault: PublicKey;
    treasuryVault: PublicKey;
};

export type StakingPoolUserAccounts = {
    userStakingState: Keypair;
    userSolAta: PublicKey;
    userHbbAta: PublicKey;
    userStablecoinAta: PublicKey;
};

export type PythPrices = {
    solPythPrice: Keypair,
    ethPythPrice: Keypair,
    btcPythPrice: Keypair,
    srmPythPrice: Keypair,
    rayPythPrice: Keypair,
    fttPythPrice: Keypair,
}

export type Env = {
    provider: anchor.Provider,
    program: anchor.Program,
    initialMarketOwner: PublicKey,
}

export function setUpProgram(): {
    initialMarketOwner: PublicKey,
    provider: anchor.Provider,
    program: anchor.Program,
    pyth: PythUtils,
} {
    let cluster = global.env.endpoint;
    const connection = new Connection(cluster, anchor.Provider.defaultOptions().commitment);
    const payer = Keypair.fromSecretKey(Buffer.from(JSON.parse(require("fs").readFileSync("./keypair.json"))));
    const wallet = new anchor.Wallet(payer);
    const provider = new anchor.Provider(connection, wallet, anchor.Provider.defaultOptions());
    const initialMarketOwner = provider.wallet.publicKey;
    anchor.setProvider(provider);
    const program = new anchor.Program(global.BorrowingIdl, global.BORROWING_PROGRAM_ID);
    const pyth = new PythUtils(provider.connection, wallet);
    return {
        initialMarketOwner,
        provider,
        program,
        pyth
    }
}

export async function setUpMint(provider: anchor.Provider, owner: PublicKey) {
    return await utils.createMint(provider, owner, 6);
}

// Creates an account owned by the program
// Build with seed such that the program can re-derive the seeds
// when it needs to transfer out of it

// Borrowing fees (in stablecoin) are taken when a loan is issued
export async function setUpBorrowingFeesAccount(
    provider: anchor.Provider,
    mint: PublicKey,
    owner: PublicKey
) {
    return await utils.createTokenAccount(provider, mint, owner);
}

// To be able to burn coins we need to transfer them to a program owned
// account, we cannot burn them straight out of people's accounts
// this account acts as a burning account
export async function setUpBurningStablecoinAccount(
    provider: anchor.Provider,
    mint: PublicKey,
    owner: PublicKey
) {
    return await utils.createTokenAccount(provider, mint, owner);
}

// A program owned SOL vault where the collateral is pooled and held until withdrawal
export async function setUpCollateralVaultAccount(
    provider: anchor.Provider,
    programId: PublicKey
): Promise<PublicKey> {
    return await utils.createSolAccount(provider, programId);
}

// Data account holding the user's state (struct)
export async function setUpUserTroveDataAccount(
    provider: anchor.Provider,
    programId: PublicKey,
    payer: PublicKey,
    signers: Array<Signer>,
    user: PublicKey,
    seed: string,
    space: number
): Promise<PublicKey> {
    return await utils.buildAccountWithSeed(
        provider,
        programId,
        payer,
        signers,
        user,
        seed,
        space
    );
}

export async function setUpAssociatedStablecoinAccount(
    provider: anchor.Provider,
    payer: PublicKey,
    user: PublicKey,
    mint: PublicKey,
    signers: Array<Signer>
): Promise<PublicKey> {
    const [ix, address] = await utils.createAssociatedTokenAccountIx(
        payer,
        user,
        mint
    );

    const tx = new Transaction();
    tx.add(ix);
    if (
        !(await utils.checkIfAccountExists(provider.connection, address))
    ) {
        await utils.send(provider, tx, payer, signers);
    }

    return address;
}

export async function setUpAssociatedTokenAccount(
    provider: anchor.Provider,
    payer: PublicKey,
    signers: Array<Signer>,
    user: PublicKey,
    mint: PublicKey
): Promise<PublicKey> {
    const [ix, address] = await utils.createAssociatedTokenAccountIx(
        payer,
        user,
        mint
    );

    const tx = new Transaction();
    tx.add(ix);

    if (
        (await utils.checkIfAccountExists(provider.connection, address)) == false
    ) {
        await utils.send(provider, tx, payer, signers);
    }

    return address;
}

export async function setUpAta(
    provider: anchor.Provider,
    user_payer: Signer,
    mint: PublicKey
): Promise<PublicKey> {
    const [ix, address] = await utils.createAssociatedTokenAccountIx(
        user_payer.publicKey,
        user_payer.publicKey,
        mint
    );

    const tx = new Transaction();
    tx.add(ix);

    if (
        (await utils.checkIfAccountExists(provider.connection, address)) == false
    ) {
        await utils.send(provider, tx, user_payer.publicKey, [user_payer]);
    }

    return address;
}


export async function setUpBorrowingGlobalAccounts(
    provider: anchor.Provider,
    initialMarketOwner: PublicKey,
    program: anchor.Program
): Promise<BorrowingGlobalAccounts> {
    const stablecoinMint = await setUpMint(provider, initialMarketOwner);
    const hbbMint = await setUpMint(provider, initialMarketOwner);

    const burningVault = await setUpBurningStablecoinAccount(
        provider,
        stablecoinMint,
        initialMarketOwner
    );

    const borrowingFeesVault = await setUpBorrowingFeesAccount(
        provider,
        stablecoinMint,
        initialMarketOwner
    );

    const ethMint = await utils.createMint(provider, initialMarketOwner);
    const btcMint = await utils.createMint(provider, initialMarketOwner);
    const srmMint = await utils.createMint(provider, initialMarketOwner);
    const rayMint = await utils.createMint(provider, initialMarketOwner);
    const fttMint = await utils.createMint(provider, initialMarketOwner);

    const collateralVaultSol = await setUpCollateralVaultAccount(provider, program.programId);
    const collateralVaultEth = await utils.createTokenAccount(provider, ethMint, initialMarketOwner);
    const collateralVaultBtc = await utils.createTokenAccount(provider, btcMint, initialMarketOwner);
    const collateralVaultSrm = await utils.createTokenAccount(provider, srmMint, initialMarketOwner);
    const collateralVaultRay = await utils.createTokenAccount(provider, rayMint, initialMarketOwner);
    const collateralVaultFtt = await utils.createTokenAccount(provider, fttMint, initialMarketOwner);

    const borrowingMarketState = new Keypair();
    const borrowingVaults = new Keypair();
    const globalConfig = new Keypair();
    const stabilityPoolState = new Keypair();
    const stakingPoolState = new Keypair();

    const redemptionsQueue = await newRedemptionsQueueAccount(provider, program);

    return {
        stablecoinMint,
        hbbMint,
        burningVault,
        borrowingFeesVault,
        borrowingMarketState,
        borrowingVaults,
        globalConfig,
        stabilityPoolState,
        stakingPoolState,
        redemptionsQueue,
        collateralVaultSol,
        collateralVaultEth,
        collateralVaultBtc,
        collateralVaultSrm,
        collateralVaultRay,
        collateralVaultFtt,
        ethMint,
        btcMint,
        srmMint,
        rayMint,
        fttMint,
    };
}

export async function newRedemptionsQueueAccount(
    provider: anchor.Provider,
    program: anchor.Program,
): Promise<PublicKey> {
    return (
        await serumCmn.createAccountRentExempt(
            provider,
            program.programId,
            program.account.redemptionsQueue.size
        )
    ).publicKey;
}

export async function setUpStabilityPoolAccounts(
    provider: anchor.Provider,
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    globalAccounts: BorrowingGlobalAccounts
): Promise<StabilityPoolAccounts> {

    let stablecoinStabilityPoolVault = await utils.createTokenAccount(
        provider,
        globalAccounts.stablecoinMint,
        initialMarketOwner
    );

    const liquidationRewardsVaultSol = await setUpCollateralVaultAccount(provider, program.programId);
    const liquidationRewardsVaultEth = await utils.createTokenAccount(provider, globalAccounts.ethMint, initialMarketOwner);
    const liquidationRewardsVaultBtc = await utils.createTokenAccount(provider, globalAccounts.btcMint, initialMarketOwner);
    const liquidationRewardsVaultSrm = await utils.createTokenAccount(provider, globalAccounts.srmMint, initialMarketOwner);
    const liquidationRewardsVaultRay = await utils.createTokenAccount(provider, globalAccounts.rayMint, initialMarketOwner);
    const liquidationRewardsVaultFtt = await utils.createTokenAccount(provider, globalAccounts.fttMint, initialMarketOwner);

    let stabilityVaults = new Keypair();

    let epochToScaleToSum = (
        await serumCmn.createAccountRentExempt(
            provider,
            program.programId,
            program.account.epochToScaleToSumAccount.size
        )
    ).publicKey;

    let liquidationsQueue = (
        await serumCmn.createAccountRentExempt(
            provider,
            program.programId,
            program.account.liquidationsQueue.size
        )
    ).publicKey;

    return {
        liquidationRewardsVaultSol,
        liquidationRewardsVaultEth,
        liquidationRewardsVaultBtc,
        liquidationRewardsVaultSrm,
        liquidationRewardsVaultRay,
        liquidationRewardsVaultFtt,
        stablecoinStabilityPoolVault,
        epochToScaleToSum,
        liquidationsQueue,
        stabilityVaults
    };
}

export async function setUpBorrowingUserAccounts(
    provider: anchor.Provider,
    payer: PublicKey,
    signers: Array<Keypair>,
    user: PublicKey,
    globalAccounts: BorrowingGlobalAccounts
): Promise<BorrowingUserAccounts> {

    return setUpBorrowingUserAccountsWithPubkeys(
        provider,
        payer,
        signers,
        user,
        globalAccounts.stablecoinMint,
        globalAccounts.ethMint,
        globalAccounts.btcMint,
        globalAccounts.srmMint,
        globalAccounts.rayMint,
        globalAccounts.fttMint,
    );

}

export async function setUpLiquidatorAccounts(
    provider: anchor.Provider,
    liquidator: Keypair,
    globalAccounts: BorrowingGlobalAccounts
): Promise<LiquidatorAccounts> {

    let ethAta = await setUpAta(provider, liquidator, globalAccounts.ethMint);
    let btcAta = await setUpAta(provider, liquidator, globalAccounts.btcMint);
    let srmAta = await setUpAta(provider, liquidator, globalAccounts.srmMint);
    let rayAta = await setUpAta(provider, liquidator, globalAccounts.rayMint);
    let fttAta = await setUpAta(provider, liquidator, globalAccounts.fttMint);

    return {
        solAta: liquidator.publicKey,
        ethAta,
        btcAta,
        srmAta,
        rayAta,
        fttAta
    };

}

export async function setUpBorrowingUserAccountsWithPubkeys(
    provider: anchor.Provider,
    payer: PublicKey,
    signers: Array<Keypair>,
    user: PublicKey,
    stablecoinMint: PublicKey,
    mintEth: PublicKey,
    mintBtc: PublicKey,
    mintSrm: PublicKey,
    mintRay: PublicKey,
    mintFtt: PublicKey,
): Promise<BorrowingUserAccounts> {
    const atas = await setUpUserAtasWithPublicKeys(
        provider,
        payer,
        signers,
        user,
        stablecoinMint,
        mintEth,
        mintBtc,
        mintSrm,
        mintRay,
        mintFtt,
    )
    return {
        ...atas,
        userMetadata: new Keypair(),
    }
}

export async function setUpUserAtas(
    provider: anchor.Provider,
    payer: PublicKey,
    signers: Array<Keypair>,
    user: PublicKey,
    globalAccounts: BorrowingGlobalAccounts,
): Promise<UserAtas> {
    return setUpUserAtasWithPublicKeys(
        provider,
        payer,
        signers,
        user,
        globalAccounts.stablecoinMint,
        globalAccounts.ethMint,
        globalAccounts.btcMint,
        globalAccounts.srmMint,
        globalAccounts.rayMint,
        globalAccounts.fttMint,
    );
}

export async function setUpUserAtasWithPublicKeys(
    provider: anchor.Provider,
    payer: PublicKey,
    signers: Array<Keypair>,
    user: PublicKey,
    stablecoinMint: PublicKey,
    mintEth: PublicKey,
    mintBtc: PublicKey,
    mintSrm: PublicKey,
    mintRay: PublicKey,
    mintFtt: PublicKey,
): Promise<UserAtas> {
    const stablecoinAta = await setUpAssociatedStablecoinAccount(
        provider,
        payer,
        user,
        stablecoinMint,
        signers
    );

    const ethAta = await setUpAta(provider, signers[0], mintEth);
    const btcAta = await setUpAta(provider, signers[0], mintBtc);
    const srmAta = await setUpAta(provider, signers[0], mintSrm);
    const rayAta = await setUpAta(provider, signers[0], mintRay);
    const fttAta = await setUpAta(provider, signers[0], mintFtt);

    return {
        stablecoinAta,
        ethAta,
        btcAta,
        srmAta,
        rayAta,
        fttAta
    };
}

export async function setUpStabilityProviderUserAccounts(
    provider: anchor.Provider,
    signers: Array<Keypair>,
    user: PublicKey,
    program: anchor.Program,
    globalAccounts: BorrowingGlobalAccounts
): Promise<StabilityProviderAccounts> {

    return setUpStabilityProviderUserAccountsWithPubkeys(
        provider,
        signers,
        user,
        program,
        globalAccounts.hbbMint,
        globalAccounts.stablecoinMint,
        globalAccounts.ethMint,
        globalAccounts.btcMint,
        globalAccounts.srmMint,
        globalAccounts.rayMint,
        globalAccounts.fttMint
    );
}

export async function setUpStabilityProviderUserAccountsWithPubkeys(
    provider: anchor.Provider,
    signers: Array<Keypair>,
    user: PublicKey,
    program: anchor.Program,
    hbbMint: PublicKey,
    stablecoinMint: PublicKey,
    mintEth: PublicKey,
    mintBtc: PublicKey,
    mintSrm: PublicKey,
    mintRay: PublicKey,
    mintFtt: PublicKey,
): Promise<StabilityProviderAccounts> {

    let hbbAta = await setUpAssociatedTokenAccount(
        provider,
        user,
        signers,
        user,
        hbbMint);

    let stablecoinAta = await setUpAssociatedTokenAccount(
        provider,
        user,
        signers,
        user,
        stablecoinMint);

    let solCollateralLiquidationsRewardsPending = await utils.buildAccountWithSeed(
        provider,
        program.programId,
        user,
        signers,
        user,
        user.toString().substring(0, 10));

    let hbbEmissionRewardsPending = await utils.createTokenAccount(
        provider,
        hbbMint,
        user);

    let stabilityProviderState = new Keypair();

    let ethAta = await setUpAta(provider, signers[0], mintEth);
    let btcAta = await setUpAta(provider, signers[0], mintBtc);
    let srmAta = await setUpAta(provider, signers[0], mintSrm);
    let rayAta = await setUpAta(provider, signers[0], mintRay);
    let fttAta = await setUpAta(provider, signers[0], mintFtt);

    return {
        hbbAta,
        stablecoinAta,
        solCollateralLiquidationsRewardsPending,
        hbbEmissionRewardsPending,
        stabilityProviderState,
        ethAta,
        btcAta,
        srmAta,
        rayAta,
        fttAta,
    }
}

export async function setUpRewardsGlobalAccounts(
    provider: anchor.Provider,
    initialMarketOwner: PublicKey,
    programId: PublicKey,
    program: anchor.Program
): Promise<RewardsGlobalAccounts> {
    // Stake mint get the same mint back
    let rewardsMint: PublicKey = await setUpMint(provider, initialMarketOwner);
    let globalStakingState: PublicKey = (
        await serumCmn.createAccountRentExempt(
            provider,
            programId,
            program.account.globalStakingState.size
        )
    ).publicKey;

    let rewardsPot: PublicKey = await utils.createTokenAccount(
        provider,
        rewardsMint,
        initialMarketOwner
    );
    let stakingVault: PublicKey = await utils.createTokenAccount(
        provider,
        rewardsMint,
        initialMarketOwner
    );

    return {
        globalStakingState,
        rewardsPot,
        stakingVault,
        rewardsMint,
    };
}


export async function updatePythPrices(
    pyth: PythUtils,
    prices: PythPrices,
    newPrices: Prices = {
        solPrice: undefined,
        ethPrice: undefined,
        btcPrice: undefined,
        srmPrice: undefined,
        fttPrice: undefined,
        rayPrice: undefined,
    }
) {
    const solPythPrice = prices.solPythPrice;
    const ethPythPrice = prices.ethPythPrice;
    const btcPythPrice = prices.btcPythPrice;
    const srmPythPrice = prices.srmPythPrice;
    const fttPythPrice = prices.fttPythPrice;
    const rayPythPrice = prices.rayPythPrice;

    let newIntPrices = toInteger(newPrices);
    const solPrice = newIntPrices.solPrice;
    const ethPrice = newIntPrices.ethPrice;
    const btcPrice = newIntPrices.btcPrice;
    const srmPrice = newIntPrices.srmPrice;
    const fttPrice = newIntPrices.fttPrice;
    const rayPrice = newIntPrices.rayPrice;

    if (solPrice) {
        await pyth.updatePriceAccount(solPythPrice, {
            exponent: -8,
            aggregatePriceInfo: {
                price: BigInt(solPrice),
            },
        });
    }
    if (ethPrice) {
        await pyth.updatePriceAccount(ethPythPrice, {
            exponent: -8,
            aggregatePriceInfo: {
                price: BigInt(ethPrice),
            },
        });
    }
    if (btcPrice) {
        await pyth.updatePriceAccount(btcPythPrice, {
            exponent: -8,
            aggregatePriceInfo: {
                price: BigInt(btcPrice),
            },
        });
    }
    if (srmPrice) {
        await pyth.updatePriceAccount(srmPythPrice, {
            exponent: -8,
            aggregatePriceInfo: {
                price: BigInt(srmPrice),
            },
        });
    }
    if (fttPrice) {
        await pyth.updatePriceAccount(fttPythPrice, {
            exponent: -8,
            aggregatePriceInfo: {
                price: BigInt(fttPrice),
            },
        });
    }
    if (rayPrice) {
        await pyth.updatePriceAccount(rayPythPrice, {
            exponent: -8,
            aggregatePriceInfo: {
                price: BigInt(rayPrice),
            },
        });
    }
}

export interface Prices {
    solPrice?: number
    ethPrice?: number
    btcPrice?: number
    srmPrice?: number
    fttPrice?: number
    rayPrice?: number
}

function toInteger(prices: Prices): Prices {
    return {
        solPrice: prices.solPrice ? prices.solPrice * 100_000_000 : 0,
        ethPrice: prices.ethPrice ? prices.ethPrice * 100_000_000 : 0,
        btcPrice: prices.btcPrice ? prices.btcPrice * 100_000_000 : 0,
        srmPrice: prices.srmPrice ? prices.srmPrice * 100_000_000 : 0,
        fttPrice: prices.fttPrice ? prices.fttPrice * 100_000_000 : 0,
        rayPrice: prices.rayPrice ? prices.rayPrice * 100_000_000 : 0,
    }
}

export async function setUpPrices(
    provider: anchor.Provider,
    pyth: PythUtils,
    prices: Prices = {
        solPrice: 228.41550900,
        ethPrice: 4726.59830000,
        btcPrice: 64622.36900000,
        srmPrice: 7.06975570,
        fttPrice: 59.17104600,
        rayPrice: 11.10038050,
    }
): Promise<PythPrices> {
    // This is the float version
    return setUpPythPrices(
        provider,
        pyth,
        toInteger(prices)
    );
}

export async function setUpPythPrices(
    provider: anchor.Provider,
    pyth: PythUtils,
    prices: Prices = {
        solPrice: 228_41550900,
        ethPrice: 4726_59830000,
        btcPrice: 64622_36900000,
        srmPrice: 7_06975570,
        fttPrice: 59_17104600,
        rayPrice: 11_10038050,
    }
): Promise<PythPrices> {

    const solPythPrice = await pyth.createPriceAccount();
    const ethPythPrice = await pyth.createPriceAccount();
    const btcPythPrice = await pyth.createPriceAccount();
    const srmPythPrice = await pyth.createPriceAccount();
    const fttPythPrice = await pyth.createPriceAccount();
    const rayPythPrice = await pyth.createPriceAccount();

    await pyth.updatePriceAccount(solPythPrice, {
        exponent: -8,
        aggregatePriceInfo: {
            price: BigInt(prices.solPrice ? prices.solPrice : 0),
        },
    });
    await pyth.updatePriceAccount(ethPythPrice, {
        exponent: -8,
        aggregatePriceInfo: {
            price: BigInt(prices.ethPrice ? prices.ethPrice : 0),
        },
    });
    await pyth.updatePriceAccount(btcPythPrice, {
        exponent: -8,
        aggregatePriceInfo: {
            price: BigInt(prices.btcPrice ? prices.btcPrice : 0),
        },
    });
    await pyth.updatePriceAccount(srmPythPrice, {
        exponent: -8,
        aggregatePriceInfo: {
            price: BigInt(prices.srmPrice ? prices.srmPrice : 0),
        },
    });
    await pyth.updatePriceAccount(fttPythPrice, {
        exponent: -8,
        aggregatePriceInfo: {
            price: BigInt(prices.fttPrice ? prices.fttPrice : 0),
        },
    });
    await pyth.updatePriceAccount(rayPythPrice, {
        exponent: -8,
        aggregatePriceInfo: {
            price: BigInt(prices.rayPrice ? prices.rayPrice : 0),
        },
    });

    return {
        solPythPrice: solPythPrice,
        ethPythPrice: ethPythPrice,
        btcPythPrice: btcPythPrice,
        srmPythPrice: srmPythPrice,
        fttPythPrice: fttPythPrice,
        rayPythPrice: rayPythPrice,
    };
}

export async function setUpRewardsUserAccounts(
    provider: anchor.Provider,
    payer: PublicKey,
    signers: Array<Keypair>,
    user: PublicKey,
    programId: PublicKey,
    program: anchor.Program,
    globalAccounts: RewardsGlobalAccounts
): Promise<RewardsUserAccounts> {
    let userStakingState: PublicKey = (
        await serumCmn.createAccountRentExempt(
            provider,
            programId,
            program.account.userStakingState.size
        )
    ).publicKey;

    let stakeRewardCoinAta: PublicKey = await setUpAssociatedTokenAccount(
        provider,
        payer,
        signers,
        user,
        globalAccounts.rewardsMint
    );

    let rewardCoinPendingPot: PublicKey = await utils.createTokenAccount(
        provider,
        globalAccounts.rewardsMint,
        user
    );

    return {
        userStakingState,
        stakeRewardCoinAta,
        rewardCoinPendingPot,
    };
}

export async function setUpStakingPoolAccounts(
    provider: anchor.Provider,
    initialMarketOwner: PublicKey,
    program: anchor.Program,
    globalAccounts: BorrowingGlobalAccounts
): Promise<StakingPoolAccounts> {
    const stakingVault: PublicKey = await utils.createTokenAccount(
        provider,
        globalAccounts.hbbMint,
        initialMarketOwner
    );

    const treasuryVault: PublicKey = await utils.createTokenAccount(
        provider,
        globalAccounts.stablecoinMint,
        initialMarketOwner
    );

    return {
        stakingVault,
        treasuryVault
    };
}

export async function setUpStakingPoolUserAccounts(
    provider: anchor.Provider,
    signers: Array<Keypair>,
    user: PublicKey,
    program: anchor.Program,
    globalAccounts: BorrowingGlobalAccounts
): Promise<StakingPoolUserAccounts> {
    return setUpStakingPoolUserAccountsWithPubkeys(
        provider,
        signers,
        user,
        globalAccounts.hbbMint,
        globalAccounts.stablecoinMint)
}

export async function setUpStakingPoolUserAccountsWithPubkeys(
    provider: anchor.Provider,
    signers: Array<Keypair>,
    user: PublicKey,
    hbbMint: PublicKey,
    stablecoinMint: PublicKey
): Promise<StakingPoolUserAccounts> {
    let userStakingState = new Keypair();

    let userSolAta = user;

    let userHbbCoinAta: PublicKey = await setUpAssociatedTokenAccount(
        provider,
        user,
        signers,
        user,
        hbbMint
    );

    let userStableCoinAta: PublicKey = await setUpAssociatedTokenAccount(
        provider,
        user,
        signers,
        user,
        stablecoinMint
    );

    return { userStakingState, userSolAta, userHbbAta: userHbbCoinAta, userStablecoinAta: userStableCoinAta };
}
