import * as anchor from '@project-serum/anchor';
import { readFileSync, writeFileSync } from 'fs';
import * as path from 'path';
import { sleep } from '@project-serum/common';
import { PublicKey } from '@solana/web3.js';
import { displayBorrowingMarketState, displayBorrowingVaults, displayStabilityPoolState, displayStabilityVaults } from '../src/utils_display';
import * as global from '../src/global';
import * as set_up from '../src/set_up';
import { BorrowingGlobalAccounts, setUpProgram, StabilityPoolAccounts, StakingPoolAccounts } from '../src/set_up';
import * as operations_borrowing from "./operations_borrowing";
import * as instructions_borrow from '../src/instructions_borrow';
import * as instructions_stability from '../src/instructions_stability';
import * as instructions_staking from '../src/instructions_staking';
import * as operations_stability from "./operations_stability";
import * as operations_staking from './operations_staking';
import * as utils from "../src/utils";
import { publicKeyReplacer, publicKeyReviver } from "../src/utils";
import { CollateralToken } from './types';
import { PythUtils } from '../src/pyth';

describe('deployment', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const configPath = path.join(__dirname, `../${global.env.cluster}-config.json`);

    it('initialize_protocol', async () => {
        let { borrowingAccounts, stabilityAccounts, stakingAccounts } = await initializeProtocol(program, provider, initialMarketOwner, 1500);
        let config = await makeConfig(program, borrowingAccounts, stabilityAccounts, stakingAccounts);
        let json = JSON.stringify(config, publicKeyReplacer, 2)
        await writeFileSync(configPath, json, { encoding: 'utf-8' });
        console.log(`Saved to ${configPath.toString()}`)
    });

    it('generate_borrowers', async () => {

        let rawConfig = await readFileSync(configPath, { encoding: 'utf-8' })
        const config: ContractsConfig = JSON.parse(rawConfig, publicKeyReviver)
        console.log(`Loaded from ${configPath.toString()}`)
        await generateBorrowers(pyth, provider, program, config, 50);
    });

    it('generate_stability_providers', async () => {

        let rawConfig = await readFileSync(configPath, { encoding: 'utf-8' })
        const config: ContractsConfig = JSON.parse(rawConfig, publicKeyReviver)
        console.log(`Loaded from ${configPath.toString()}`)
        await generateStabilityProviders(provider, program, config, 20);
    });

    it('generate_stakers', async () => {
        let rawConfig = await readFileSync(configPath, { encoding: 'utf-8' })
        const config: ContractsConfig = JSON.parse(rawConfig, publicKeyReviver)
        console.log(`Loaded from ${configPath.toString()}`)
        await generateStakers(
            provider,
            program,
            config,
            initialMarketOwner,
            30);
    });
});


type ContractsConfig = {
    stablecoinMint: string,
    stablecoinMintAuthority: string,
    hbbMintAuthority: string,
    redemptionsQueue: string,
    borrowingMarketState: string,
    stabilityPoolState: string,
    mint: {
        ETH: string,
        BTC: string,
        SRM: string,
        RAY: string,
        FTT: string,
        HBB: string,
    },
    collateralVaultsAuthority: string,
    collateralVault: {
        SOL: string,
        ETH: string,
        BTC: string,
        SRM: string,
        RAY: string,
        FTT: string,
    },
    epochToScaleToSum: string,
    stablecoinStabilityPoolVault: string,
    stabilityVaults: string,
    stablecoinStabilityPoolVaultAuthority: string,
    liquidationRewardsVaultSol: string,
    liquidationRewardsVaultSrm: string,
    liquidationRewardsVaultEth: string,
    liquidationRewardsVaultBtc: string,
    liquidationRewardsVaultRay: string,
    liquidationRewardsVaultFtt: string,
    liquidationRewardsVaultAuthority: string,
    liquidationsQueue: string,
    borrowingVaults: string,
    stakingPoolState: string,
    borrowingFeesAccount: string,
    borrowingFeesVaultAuthority: string,
    stakingVault: string,
    treasuryVault: string,
    burningVault: string,
    burningVaultAuthority: string
}

async function makeConfig(
    program: anchor.Program,
    borrowingAccounts: set_up.BorrowingGlobalAccounts,
    stabilityAccounts: set_up.StabilityPoolAccounts,
    stakingAccounts: set_up.StakingPoolAccounts
): Promise<ContractsConfig> {

    const stabilityVaultsAcc: any = await program.account.stabilityVaults.fetch(stabilityAccounts.stabilityVaults.publicKey);
    const borrowingVaultsAcc: any = await program.account.borrowingVaults.fetch(borrowingAccounts.borrowingVaults.publicKey);
    const borrowingMarketStateAcc: any = await program.account.borrowingMarketState.fetch(borrowingAccounts.borrowingMarketState.publicKey);

    let config: ContractsConfig = {
        stablecoinMint: borrowingAccounts.stablecoinMint.toString(),
        stablecoinMintAuthority: borrowingMarketStateAcc.stablecoinMintAuthority.toString(),
        hbbMintAuthority: borrowingMarketStateAcc.hbbMintAuthority.toString(),
        mint: {
            ETH: borrowingAccounts.ethMint.toString(),
            BTC: borrowingAccounts.btcMint.toString(),
            SRM: borrowingAccounts.srmMint.toString(),
            RAY: borrowingAccounts.rayMint.toString(),
            FTT: borrowingAccounts.fttMint.toString(),
            HBB: borrowingAccounts.hbbMint.toString(),
        },
        collateralVaultsAuthority: borrowingVaultsAcc.collateralVaultsAuthority.toString(),
        collateralVault: {
            SOL: borrowingAccounts.collateralVaultSol.toString(),
            ETH: borrowingAccounts.collateralVaultEth.toString(),
            BTC: borrowingAccounts.collateralVaultBtc.toString(),
            SRM: borrowingAccounts.collateralVaultSrm.toString(),
            RAY: borrowingAccounts.collateralVaultRay.toString(),
            FTT: borrowingAccounts.collateralVaultFtt.toString()
        },
        redemptionsQueue: borrowingAccounts.redemptionsQueue.toString(),
        liquidationsQueue: stabilityAccounts.liquidationsQueue.toString(),
        borrowingMarketState: borrowingAccounts.borrowingMarketState.publicKey.toString(),
        stabilityPoolState: borrowingAccounts.stabilityPoolState.publicKey.toString(),
        epochToScaleToSum: stabilityAccounts.epochToScaleToSum.toString(),
        stablecoinStabilityPoolVault: stabilityAccounts.stablecoinStabilityPoolVault.toString(),
        stabilityVaults: stabilityAccounts.stabilityVaults.publicKey.toString(),
        stablecoinStabilityPoolVaultAuthority: stabilityVaultsAcc.stablecoinStabilityPoolVaultAuthority,
        liquidationRewardsVaultSol: stabilityVaultsAcc.liquidationRewardsVaultSol,
        liquidationRewardsVaultSrm: stabilityVaultsAcc.liquidationRewardsVaultSrm,
        liquidationRewardsVaultEth: stabilityVaultsAcc.liquidationRewardsVaultEth,
        liquidationRewardsVaultBtc: stabilityVaultsAcc.liquidationRewardsVaultBtc,
        liquidationRewardsVaultRay: stabilityVaultsAcc.liquidationRewardsVaultRay,
        liquidationRewardsVaultFtt: stabilityVaultsAcc.liquidationRewardsVaultFtt,
        liquidationRewardsVaultAuthority: stabilityVaultsAcc.liquidationRewardsVaultAuthority,
        borrowingVaults: borrowingAccounts.borrowingVaults.publicKey.toString(),
        stakingPoolState: borrowingAccounts.stakingPoolState.publicKey.toString(),
        borrowingFeesAccount: borrowingAccounts.borrowingFeesVault.toString(),
        borrowingFeesVaultAuthority: borrowingVaultsAcc.borrowingFeesVaultAuthority,
        stakingVault: stakingAccounts.stakingVault.toString(),
        treasuryVault: stakingAccounts.treasuryVault.toString(),
        burningVault: borrowingAccounts.burningVault.toString(),
        burningVaultAuthority: borrowingVaultsAcc.burningVaultAuthority

    }

    return config;
}

async function generateBorrowers(
    pyth: PythUtils,
    provider: anchor.Provider,
    program: anchor.Program,
    config: ContractsConfig,
    numUsers: number) {
    const randInt = (min: number, max: number): number => { // min and max included
        return Math.floor(Math.random() * (max - min + 1) + min);
    };

    const pythPrices = await set_up.setUpPythPrices(provider, pyth);


    let borrowAmount = 200_000.00;
    let maxAmount = 5;

    let totalDeposited: { [key: string]: number } = {
        "SOL": 0,
        "ETH": 0,
        "BTC": 0,
        "FTT": 0,
        "SRM": 0,
        "RAY": 0,
    }

    for (let i = 0; i < numUsers; i++) {
        console.log("User ", i + 1);
        const {
            borrower,
            borrowerAccounts
        } = await operations_borrowing.newBorrowingUserWithPubkeys(
            provider,
            program,
            0.01,
            new PublicKey(config.borrowingMarketState),
            new PublicKey(config.stablecoinMint),
            new PublicKey(config.mint.ETH),
            new PublicKey(config.mint.BTC),
            new PublicKey(config.mint.SRM),
            new PublicKey(config.mint.RAY),
            new PublicKey(config.mint.FTT));

        let deposits: { [key: string]: number } = {
            "SOL": 0, // randInt(0, maxAmount),
            "ETH": randInt(0, maxAmount - 1),
            "BTC": randInt(0, maxAmount - 1),
            "FTT": randInt(0, maxAmount * 10),
            "SRM": randInt(0, maxAmount * 10),
            "RAY": randInt(0, maxAmount * 10),
        }

        await utils.mintTo(provider, new PublicKey(config.mint.SRM), borrowerAccounts.srmAta, utils.collToLamports(maxAmount * 10, "SRM"));
        await utils.mintTo(provider, new PublicKey(config.mint.BTC), borrowerAccounts.btcAta, utils.collToLamports(maxAmount * 10, "BTC"));
        await utils.mintTo(provider, new PublicKey(config.mint.RAY), borrowerAccounts.rayAta, utils.collToLamports(maxAmount * 10, "RAY"));
        await utils.mintTo(provider, new PublicKey(config.mint.ETH), borrowerAccounts.ethAta, utils.collToLamports(maxAmount * 10, "ETH"));
        await utils.mintTo(provider, new PublicKey(config.mint.FTT), borrowerAccounts.fttAta, utils.collToLamports(maxAmount * 10, "FTT"));

        for (let token in deposits) {
            let amount = deposits[token];
            totalDeposited[token] += deposits[token];

            if (amount <= 0) {
                continue;
            }

            await operations_borrowing.depositCollateralWithPubkey(
                provider,
                program,
                amount,
                borrower,
                borrowerAccounts,
                new PublicKey(config.borrowingMarketState),
                new PublicKey(config.borrowingVaults),
                new PublicKey(config.collateralVault.SOL),
                new PublicKey(config.collateralVault.ETH),
                new PublicKey(config.collateralVault.BTC),
                new PublicKey(config.collateralVault.SRM),
                new PublicKey(config.collateralVault.RAY),
                new PublicKey(config.collateralVault.FTT),
                new PublicKey(config.stablecoinMint),
                new PublicKey(config.mint.ETH),
                new PublicKey(config.mint.BTC),
                new PublicKey(config.mint.SRM),
                new PublicKey(config.mint.RAY),
                new PublicKey(config.mint.FTT),
                new PublicKey(config.mint.HBB),
                token as CollateralToken);
        }

        try {
            await operations_borrowing.borrowWithPubkeys(
                program,
                borrower,
                borrowerAccounts,
                new PublicKey(config.stablecoinMint),
                new PublicKey(config.borrowingMarketState),
                new PublicKey(config.borrowingVaults),
                new PublicKey(config.stakingPoolState),
                new PublicKey(config.borrowingFeesAccount),
                new PublicKey(config.treasuryVault),
                pythPrices,
                borrowAmount,
            );
        } catch (e) {
            console.error("Couldn't borrow..", e);
        }
    }
}

async function generateStabilityProviders(
    provider: anchor.Provider,
    program: anchor.Program,
    config: ContractsConfig,
    numUsers: number) {
    let stabilityPoolDeposit = 200_000.00;
    for (let i = 0; i < numUsers; i++) {
        console.log("User ", i + 1);

        // Provide stability
        await operations_stability.newStabilityProviderWithPubkeys(
            provider,
            program,
            new PublicKey(config.stabilityVaults),
            new PublicKey(config.epochToScaleToSum),
            new PublicKey(config.stablecoinStabilityPoolVault),
            new PublicKey(config.borrowingMarketState),
            new PublicKey(config.stablecoinMint),
            new PublicKey(config.stabilityPoolState),
            new PublicKey(config.mint.HBB),
            new PublicKey(config.mint.ETH),
            new PublicKey(config.mint.BTC),
            new PublicKey(config.mint.SRM),
            new PublicKey(config.mint.RAY),
            new PublicKey(config.mint.FTT),
            stabilityPoolDeposit,
        );

    }
}

async function generateStakers(
    provider: anchor.Provider,
    program: anchor.Program,
    config: ContractsConfig,
    initialMarketOwner: PublicKey,
    numUsers: number) {
    let hbbToStake = utils.decimalToU64(1000);
    for (let i = 0; i < numUsers; i++) {
        console.log("User ", i + 1);

        // Provide stability
        await operations_staking.newStakingPoolUserWithPubkeys(
            provider,
            program,
            initialMarketOwner,
            hbbToStake,
            new PublicKey(config.borrowingMarketState),
            new PublicKey(config.stakingVault),
            new PublicKey(config.stakingPoolState),
            new PublicKey(config.mint.HBB),
            new PublicKey(config.stablecoinMint));
    }
}


async function initializeProtocol(
    program: anchor.Program,
    provider: anchor.Provider,
    initialMarketOwner: PublicKey,
    treasuryFeeRate: number,
): Promise<{
    borrowingAccounts: BorrowingGlobalAccounts,
    stabilityAccounts: StabilityPoolAccounts,
    stakingAccounts: StakingPoolAccounts
}> {
    const borrowingAccounts = await set_up.setUpBorrowingGlobalAccounts(
        provider,
        initialMarketOwner,
        program);

    const stabilityAccounts = await set_up.setUpStabilityPoolAccounts(
        provider,
        program,
        initialMarketOwner,
        borrowingAccounts
    );

    const stakingAccounts = await set_up.setUpStakingPoolAccounts(
        provider,
        initialMarketOwner,
        program,
        borrowingAccounts
    );

    await instructions_borrow
        .initializeBorrowingMarket(
            program,
            initialMarketOwner,
            borrowingAccounts
        );

    await instructions_stability
        .initializeStabilityPool(
            program,
            initialMarketOwner,
            borrowingAccounts,
            stabilityAccounts
        );

    await instructions_staking
        .initializeStakingPool(
            program,
            initialMarketOwner,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stakingPoolState,
            stakingAccounts.stakingVault,
            stakingAccounts.treasuryVault,
            treasuryFeeRate
        )

    await sleep(1000);


    console.log('Initialized market');
    console.log(`ProgramId ${program.programId.toString()}`);

    console.log("Borrowing Accounts");
    for (let key in borrowingAccounts) {
        // @ts-ignore
        let value = borrowingAccounts[key].publicKey == undefined ? borrowingAccounts[key] : borrowingAccounts[key].publicKey;
        console.log(`Borrowing accounts ${key} ${value.toString()}`);
    }

    for (let key in stakingAccounts) {
        // @ts-ignore
        let value = stakingAccounts[key].publicKey == undefined ? stakingAccounts[key] : stakingAccounts[key].publicKey;
        console.log(`Staking accounts ${key} ${value.toString()}`);
    }

    for (let key in stabilityAccounts) {
        // @ts-ignore
        let value = stabilityAccounts[key].publicKey == undefined ? stabilityAccounts[key] : stabilityAccounts[key].publicKey;
        console.log(`Stability accounts ${key} ${value.toString()}`);
    }

    // Also create Stablecoin ATA, mint some
    // Also create HBB ata, mint some

    // let stablecoinATA = await set_up.setUpAssociatedTokenAccount(provider, user.publicKey, [user], provider.wallet.publicKey, borrowingAccounts.stablecoinMint);
    // let hbbATA = await set_up.setUpAssociatedTokenAccount(provider, user.publicKey, [user], provider.wallet.publicKey, borrowingAccounts.hbbMint);

    // console.log("Wallet Stablecoin ATA", stablecoinATA.toString(), borrowingAccounts.stablecoinMint.toString());
    // console.log("Wallet HBB ATA", hbbATA.toString(), borrowingAccounts.hbbMint.toString());

    await displayBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey);
    await displayStabilityPoolState(program, borrowingAccounts.stabilityPoolState.publicKey);
    await displayBorrowingVaults(program, borrowingAccounts.borrowingVaults.publicKey);
    await displayStabilityVaults(program, stabilityAccounts.stabilityVaults.publicKey);

    return { borrowingAccounts, stabilityAccounts, stakingAccounts };

}