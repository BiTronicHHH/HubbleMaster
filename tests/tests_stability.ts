import * as anchor from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';
import * as set_up from '../src/set_up';
import { setUpProgram } from '../src/set_up';
import * as utils from '../src/utils';
import { decimalToU64, u64ToDecimal } from '../src/utils';

import * as instructions_borrow from '../src/instructions_borrow';
import * as instructions_stability from '../src/instructions_stability';
import * as operations_borrowing from "./operations_borrowing";
import { newLoanee } from "./operations_borrowing";
import * as operations_stability from "./operations_stability";
import { createMarketAndStabilityPool, newLiquidator } from "./operations_stability";

import { getBorrowingMarketState, getForcedSolBalanceInLamports, getStabilityPoolState, getTokenAccountBalance } from './data_provider';
import * as assert from "assert";
import { displayBorrowingMarketState, displayStabilityPoolState } from '../src/utils_display';
import { assertBorrowerBalance, assertBorrowerCollateral, assertGlobalCollateral, assertGlobalDebt, assertStabilityPool, assertStabilityProviderBalance } from './test_assertions';
import { sleep } from '@project-serum/common';
import { CollateralToken } from "./types";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'

chai.use(chaiAsPromised)

describe('tests_stability', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('tests_stability_initialize_market', async () => {
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
                stabilityAccounts,
            );

        await displayBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey);
        await displayStabilityPoolState(program, borrowingAccounts.stabilityPoolState.publicKey);

        const stabilityPoolState = await getStabilityPoolState(program, borrowingAccounts.stabilityPoolState.publicKey);
        const borrowingMarketState = await getBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey);

        assert.strictEqual(
            stabilityPoolState.borrowingMarketState.toBase58(),
            borrowingAccounts.borrowingMarketState.publicKey.toBase58()
        );
        assert.strictEqual(
            stabilityPoolState.epochToScaleToSum.toBase58(),
            stabilityAccounts.epochToScaleToSum.toBase58()
        );
        assert.strictEqual(
            stabilityPoolState.liquidationsQueue.toBase58(),
            stabilityAccounts.liquidationsQueue.toBase58()
        );
        assert.strictEqual(stabilityPoolState.numUsers, 0);
        assert.strictEqual(stabilityPoolState.stablecoinDeposited, 0);

        assert.strictEqual(borrowingMarketState.numUsers, 0);
        assert.strictEqual(borrowingMarketState.stablecoinBorrowed, 0);
    });


    it('tests_stability_approve_stability', async () => {
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await operations_stability.createMarketAndStabilityPool(env);

        const { keyPair: user } = await utils.solAccountWithMinBalance(provider, 15);

        console.log("user", user.publicKey.toString());
        const userStabilityProviderAccounts = await set_up.setUpStabilityProviderUserAccounts(
            provider,
            [user],
            user.publicKey,
            program,
            borrowingAccounts
        );

        await instructions_stability.approveStability(
            program,
            user.publicKey,
            userStabilityProviderAccounts.stabilityProviderState,
            borrowingAccounts.stabilityPoolState.publicKey,
            [user]
        );

        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts,
            1, 0, 0);
        await assertStabilityProviderBalance(provider, program, user.publicKey, borrowingAccounts, userStabilityProviderAccounts,
            0, 0);
    });

    it('tests_stability_provide_stability', async () => {
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await operations_stability.createMarketAndStabilityPool(env);

        const { keyPair: user } = await utils.solAccountWithMinBalance(provider, 15);

        console.log("user", user.publicKey.toString());
        const userStabilityProviderAccounts = await set_up.setUpStabilityProviderUserAccounts(
            provider,
            [user],
            user.publicKey,
            program,
            borrowingAccounts
        );

        await instructions_stability.approveStability(
            program,
            user.publicKey,
            userStabilityProviderAccounts.stabilityProviderState,
            borrowingAccounts.stabilityPoolState.publicKey,
            [user]
        );

        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts, 1, 0, 0);
        await assertStabilityProviderBalance(provider, program, user.publicKey, borrowingAccounts, userStabilityProviderAccounts, 0, 0);

        const stablecoinToProvide = 10;
        // HACK: we mint/airdrop it now, later we should borrow it against coins and then provide it
        await instructions_borrow.airdropStablecoin(
            program,
            initialMarketOwner,
            borrowingAccounts.borrowingMarketState.publicKey,
            userStabilityProviderAccounts.stablecoinAta,
            borrowingAccounts.stablecoinMint,
            utils.decimalToU64(stablecoinToProvide),
        );

        await assertStabilityProviderBalance(provider, program, user.publicKey, borrowingAccounts, userStabilityProviderAccounts, 0, stablecoinToProvide);

        await instructions_stability.provideStability(
            program,
            user.publicKey,
            userStabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            userStabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [user]
        );

        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts,
            1, stablecoinToProvide, 0);
        await assertStabilityProviderBalance(provider, program, user.publicKey, borrowingAccounts, userStabilityProviderAccounts,
            stablecoinToProvide, 0);
    });

    it('tests_stability_withdraw_stability', async () => {
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await operations_stability.createMarketAndStabilityPool(env);

        const { keyPair: user } = await utils.solAccountWithMinBalance(provider, 15);
        console.log("user", user.publicKey.toString());
        const userStabilityProviderAccounts = await set_up.setUpStabilityProviderUserAccounts(
            provider,
            [user],
            user.publicKey,
            program,
            borrowingAccounts
        );

        await instructions_stability.approveStability(
            program,
            user.publicKey,
            userStabilityProviderAccounts.stabilityProviderState,
            borrowingAccounts.stabilityPoolState.publicKey,
            [user]
        );

        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts,
            1, 0, 0);
        await assertStabilityProviderBalance(provider, program, user.publicKey, borrowingAccounts, userStabilityProviderAccounts,
            0, 0);

        const stablecoinToProvide = 10;
        // HACK: we mint/airdrop it now, later we should borrow it against coins and then provide it
        await instructions_borrow.airdropStablecoin(
            program,
            initialMarketOwner,
            borrowingAccounts.borrowingMarketState.publicKey,
            userStabilityProviderAccounts.stablecoinAta,
            borrowingAccounts.stablecoinMint,
            utils.decimalToU64(stablecoinToProvide),
        );

        await instructions_stability.provideStability(
            program,
            user.publicKey,
            userStabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            userStabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(stablecoinToProvide),
            [user]
        );

        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts,
            1, stablecoinToProvide, 0);
        await assertStabilityProviderBalance(provider, program, user.publicKey, borrowingAccounts, userStabilityProviderAccounts,
            stablecoinToProvide, 0);

        const stablecoinToWithdraw = 9;
        await instructions_stability.withdrawStability(
            program,
            user.publicKey,
            userStabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            userStabilityProviderAccounts.stablecoinAta,
            decimalToU64(stablecoinToWithdraw),
            [user]
        );

        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts,
            1, 1, 0);
        await assertStabilityProviderBalance(provider, program, user.publicKey, borrowingAccounts, userStabilityProviderAccounts,
            1, stablecoinToWithdraw);
    });

    it('tests_stability_provide_liquidate_noharvest', async () => {

        // SOL/USD is 40.0
        // Borrower deposits 10 SOL (400 USDH), borrows 300 USDH, coll ratio 133%
        // Depositor deposits 400 USDH
        // Force liquidation at price of SOL/USD 30.0
        // Borrower has nothing left, only a 300 USDH balance in the wallet
        // Stability pool has 100 USDH left, and a gain of 10 SOL

        const borrowerSolDeposit = 10;
        const borrowerStablecoinBorrow = 300;
        const stabilityPoolDeposit = 400;

        // Set up global accounts
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await createMarketAndStabilityPool(env);

        const pythPrices = await set_up.setUpPrices(provider, pyth, { solPrice: 40.0 });

        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts, 0, 0, 0);

        const extraSolDeposit = 200;
        const extraDebt = 200;

        // we need one more, cannot liquidate the last user
        // this user needs to be well collateralized
        await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, extraDebt, new Map<CollateralToken, number>([
            ["SOL", extraSolDeposit]
        ]));

        // Deposit & Borrow
        const { borrower, borrowerAccounts, borrowerInitialBalance } = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, borrowerStablecoinBorrow, new Map<CollateralToken, number>([
            ["SOL", borrowerSolDeposit]
        ]));


        const expectedStablecoinDebt = 301.5;
        const expectedStablecoinGlobalDebt = 502.5;
        await assertGlobalCollateral(program, provider, borrowingAccounts.borrowingMarketState.publicKey, borrowingAccounts.borrowingVaults.publicKey, new Map<CollateralToken, number>([
            ["SOL", borrowerSolDeposit + extraSolDeposit]
        ]));

        await assertGlobalDebt(program, borrowingAccounts.borrowingMarketState.publicKey, expectedStablecoinGlobalDebt);
        await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingAccounts,
            expectedStablecoinDebt,
            borrowerInitialBalance - borrowerSolDeposit,
            borrowerStablecoinBorrow);
        await assertBorrowerCollateral(provider, program, borrower, borrowerAccounts, borrowingAccounts, new Map<CollateralToken, number>([
            ["SOL", borrowerSolDeposit]
        ]))

        // Provide stability
        const { stabilityProvider, stabilityProviderAccounts } = await operations_stability.newStabilityProvider(
            provider,
            program,
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityPoolDeposit,
        );

        console.log("Provided stability");
        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts,
            1, stabilityPoolDeposit, 0);
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            stabilityPoolDeposit, 0);

        const { liquidator, liquidatorAccounts, } = await newLiquidator(provider, program, borrowingAccounts);

        await displayStabilityPoolState(program, borrowingAccounts.stabilityPoolState.publicKey);
        await sleep(500);

        const liquidatorBalanceBeforeLiquidation = await getForcedSolBalanceInLamports(provider, liquidator.publicKey);

        console.log("Provided stability");

        // SOL price drops to 30 USD/SOL
        // 301.5 * 1.1 = 331.65000000000003
        // 331.65000000000003 / 10 = 33.165000000000006
        const reducedPythPrices = await set_up.setUpPrices(provider, pyth, { solPrice: 33.0 });

        await operations_stability.tryLiquidate(program, liquidator, borrowingAccounts, stabilityPoolAccounts, borrowerAccounts, liquidatorAccounts, reducedPythPrices);

        const liquidatorFee = 0.05; // 0.5% of 10
        await sleep(500);
        const stabilityPoolRemaining = stabilityPoolDeposit - expectedStablecoinDebt;
        await assertGlobalCollateral(program, provider, borrowingAccounts.borrowingMarketState.publicKey, borrowingAccounts.borrowingVaults.publicKey, new Map<CollateralToken, number>([
            ["SOL", extraSolDeposit]
        ]));

        await assertGlobalDebt(program, borrowingAccounts.borrowingMarketState.publicKey, expectedStablecoinGlobalDebt - expectedStablecoinDebt);
        await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingAccounts,
            0,
            borrowerInitialBalance - borrowerSolDeposit,
            borrowerStablecoinBorrow);
        await assertBorrowerCollateral(provider, program, borrower, borrowerAccounts, borrowingAccounts, new Map<CollateralToken, number>([
            ["SOL", 0]
        ]))

        await assertStabilityPool(program, provider, borrowingAccounts.stabilityPoolState.publicKey, stabilityPoolAccounts, 1, stabilityPoolRemaining, borrowerSolDeposit - liquidatorFee);

        // provided stability is still 400, because the state is not recalculated/updated
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            stabilityPoolDeposit, 0);

        const liquidatorBalanceAfterLiquidation = await getForcedSolBalanceInLamports(provider, liquidator.publicKey);
        const liquidatorDiff = liquidatorBalanceAfterLiquidation - liquidatorBalanceBeforeLiquidation;
        assert.strictEqual(liquidatorDiff, utils.collToLamports(liquidatorFee, "SOL"));

        await displayStabilityPoolState(program, borrowingAccounts.stabilityPoolState.publicKey);

        console.log("stabilityPoolAccounts.epochToScaleToSum", stabilityPoolAccounts.epochToScaleToSum.toString());

        // printEpoch(stabilityPoolAccounts.epochToScaleToSum);

    });

    it('tests_stability_provide_liquidate_harvest', async () => {

        // SOL/USD is 40.0
        // Borrower deposits 10 SOL (400 USDH), borrows 300 USDH, coll ratio 133%
        // Depositor deposits 400 USDH
        // Force liquidation at price of SOL/USD 30.0
        // Borrower has nothing left, only a 300 USDH balance in the wallet
        // Stability pool has 100 USDH left, and a gain of 10 SOL

        const borrowerSolDeposit = 10;
        const expectedLiquiationGains = 9.95; // 0.5 is fee
        const borrowerStablecoinBorrow = 300;
        const stabilityPoolDeposit = 400;

        // Set up global accounts
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await createMarketAndStabilityPool(env);

        const pythPrices = await set_up.setUpPrices(provider, pyth, { solPrice: 40.0 });

        // we need one more, cannot liquidate the last user
        await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, borrowerStablecoinBorrow, new Map<CollateralToken, number>([
            ["SOL", borrowerSolDeposit + 100]
        ]));

        // Deposit & Borrow
        const { borrower, borrowerAccounts } = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, borrowerStablecoinBorrow, new Map<CollateralToken, number>([
            ["SOL", borrowerSolDeposit]
        ]));

        // Provide stability
        const { stabilityProvider, stabilityProviderAccounts } = await operations_stability.newStabilityProvider(
            provider,
            program,
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityPoolDeposit,
        );

        const { liquidator, liquidatorAccounts, } = await newLiquidator(provider, program, borrowingAccounts);

        // SOL price drops to 33 USD/SOL (109% CR)
        const reducedPythPrices = await set_up.setUpPrices(provider, pyth, { solPrice: 33.0 });

        // Liquidate
        await operations_stability.tryLiquidate(program, liquidator, borrowingAccounts, stabilityPoolAccounts, borrowerAccounts, liquidatorAccounts, reducedPythPrices);
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            stabilityPoolDeposit, 0);

        // Harvest
        const balanceBeforeHarvest = await getForcedSolBalanceInLamports(provider, stabilityProvider.publicKey);
        await operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts,
            "SOL");
        const balanceAfterHarvest = await getForcedSolBalanceInLamports(provider, stabilityProvider.publicKey);
        console.log(`balanceBeforeHarvest`, balanceBeforeHarvest);
        console.log(`balanceAfterHarvest`, balanceAfterHarvest);

        assert.strictEqual(balanceAfterHarvest - balanceBeforeHarvest, utils.collToLamports(expectedLiquiationGains, "SOL"));
        // doing -1 cause we always remove one decimal extra in the stability pool calculations
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            u64ToDecimal(decimalToU64(stabilityPoolDeposit - borrowerStablecoinBorrow * 1.005) - 1), 0);

    });

    it('tests_stability_provide_liquidate_multi_collateral_harvest', async () => {

        // Starting prices:
        // SOL/USD -> 10.0
        // ETH/USD -> 10.0
        // SRM/USD -> 10.0
        // RAY/USD -> 10.0
        // FTT/USD -> 10.0
        //
        // Borrower deposits 10 SOL, 2 ETH, 7 SRM, 9 RAY + 12 FTT (400 USDH), borrows 300 USDH, coll ratio 133%
        // Stability Depositor deposits 400 USDH
        // Last borrower deposits 1000 SOL, takes a loan of 2000 -> 
        //
        // Prices drop:
        // SOL/USD -> 7.6
        // SRM/USD -> 7.6
        // RAY/USD -> 7.6
        // FTT/USD -> 7.6
        // 
        // Coll Ratio: 
        // Coins amonts = 10 + 2 + 7 + 9 + 12 = 40
        // CR > 100 means (such that we liquidate with SP, not redistribution) 300 / 40 = 7.5,  
        // MV = 40 * 7.6 = 304
        // Borrow = 300 -> CR = 304 / 300 = 1.0133333333333334
        //
        // Force liquidation
        // Borrower has nothing left, only a 300 USDH balance in the wallet
        // Stability pool has 100 USDH left, and a gain of 10 SOL

        const borrowerSolDeposit = 10;
        const borrowerEthDeposit = 2;
        const borrowerSrmDeposit = 7;
        const borrowerRayDeposit = 9;
        const borrowerFttDeposit = 12;
        const expectedLiquiationGains = 9.95; // 0.5 is fee
        const borrowerStablecoinBorrow = 300;
        const stabilityPoolDeposit = 400;

        const pythPrices = await set_up.setUpPrices(provider, pyth,
            {
                solPrice: 10.0,
                ethPrice: 10.0,
                btcPrice: 10.0,
                srmPrice: 10.0,
                rayPrice: 10.0,
                fttPrice: 10.0,
            }
        );

        // Set up global accounts
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await createMarketAndStabilityPool(env);

        // we need one more, cannot liquidate the last user
        await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
            ["SOL", 1000]
        ]));

        const { borrower, borrowerAccounts } = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, borrowerStablecoinBorrow, new Map<CollateralToken, number>([
            ["SOL", borrowerSolDeposit],
            ["ETH", borrowerEthDeposit],
            ["RAY", borrowerRayDeposit],
            ["SRM", borrowerSrmDeposit],
            ["FTT", borrowerFttDeposit],
        ]));

        // Provide stability
        const { stabilityProvider, stabilityProviderAccounts } = await operations_stability.newStabilityProvider(
            provider,
            program,
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityPoolDeposit,
        );

        const { liquidator, liquidatorAccounts, } = await newLiquidator(provider, program, borrowingAccounts);

        // prices drop
        const reducedPythPrices = await set_up.setUpPrices(provider, pyth, {
            solPrice: 7.6,
            ethPrice: 7.6,
            btcPrice: 7.6,
            srmPrice: 7.6,
            rayPrice: 7.6,
            fttPrice: 7.6,
        });

        // Liquidate
        await operations_stability.tryLiquidate(program, liquidator, borrowingAccounts, stabilityPoolAccounts, borrowerAccounts, liquidatorAccounts, reducedPythPrices);
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            stabilityPoolDeposit, 0);

        // Harvest
        const balanceBeforeHarvestSol = await getForcedSolBalanceInLamports(provider, stabilityProvider.publicKey);
        const balanceBeforeHarvestEth = await getTokenAccountBalance(program, stabilityProviderAccounts.ethAta);
        const balanceBeforeHarvestSrm = await getTokenAccountBalance(program, stabilityProviderAccounts.srmAta);
        const balanceBeforeHarvestRay = await getTokenAccountBalance(program, stabilityProviderAccounts.rayAta);
        const balanceBeforeHarvestFtt = await getTokenAccountBalance(program, stabilityProviderAccounts.fttAta);

        await operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts, "SOL");
        await operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts, "ETH");
        await operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts, "SRM");
        await operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts, "RAY");
        await operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts, "FTT");

        await sleep(1000);

        console.log("stabilityProviderAccounts.ethAta", stabilityProviderAccounts.ethAta.toString());
        console.log("stabilityProvider.publicKey", stabilityProvider.publicKey.toString());
        const balanceAfterHarvestSol = await getForcedSolBalanceInLamports(provider, stabilityProvider.publicKey);
        const balanceAfterHarvestEth = await getTokenAccountBalance(program, stabilityProviderAccounts.ethAta);
        const balanceAfterHarvestSrm = await getTokenAccountBalance(program, stabilityProviderAccounts.srmAta);
        const balanceAfterHarvestRay = await getTokenAccountBalance(program, stabilityProviderAccounts.rayAta);
        const balanceAfterHarvestFtt = await getTokenAccountBalance(program, stabilityProviderAccounts.fttAta);

        console.log(`Before Harvest SOL ${balanceBeforeHarvestSol} After Harvest SOL ${balanceAfterHarvestSol} `);
        console.log(`Before Harvest ETH ${balanceBeforeHarvestEth} After Harvest ETH ${balanceAfterHarvestEth} `);
        console.log(`Before Harvest SRM ${balanceBeforeHarvestSrm} After Harvest SRM ${balanceAfterHarvestSrm} `);
        console.log(`Before Harvest RAY ${balanceBeforeHarvestRay} After Harvest RAY ${balanceAfterHarvestRay} `);
        console.log(`Before Harvest FTT ${balanceBeforeHarvestFtt} After Harvest FTT ${balanceAfterHarvestFtt} `);

        assert.strictEqual(balanceAfterHarvestSol - balanceBeforeHarvestSol, utils.collToLamports(expectedLiquiationGains, "SOL"));
        assert.strictEqual(balanceAfterHarvestEth, 1.99); // 2 * 0.995
        assert.strictEqual(balanceAfterHarvestSrm, 6.965); // 7 * 0.995
        assert.strictEqual(balanceAfterHarvestRay, 8.955); // 9 * 0.995 = 8.955
        assert.strictEqual(balanceAfterHarvestFtt, 11.94); // 12 * 0.995 = 11.94

        // doing -1 cause we always remove one decimal extra in the stability pool calculations
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            u64ToDecimal(decimalToU64(stabilityPoolDeposit - borrowerStablecoinBorrow * 1.005) - 1), 0);
    });

    it('tests_stability_hbb_staking_provide_harvest', async () => {

        // SOL/USD is 40.0
        // Borrower deposits 10 SOL (400 USDH), borrows 300 USDH, coll ratio 133%
        // Depositor deposits 400 USDH
        // Force liquidation at price of SOL/USD 30.0
        // Borrower has nothing left, only a 300 USDH balance in the wallet
        // Stability pool has 100 USDH left, and a gain of 10 SOL

        const stabilityPoolDeposit = 100;

        // Set up global accouunts
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await createMarketAndStabilityPool(env);

        // Provide stability
        // triggers no emission event
        const stabilityProviderAccountsOne = await operations_stability.newStabilityProvider(
            provider,
            program,
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityPoolDeposit,
        );

        // triggers one emission event -> user one gets 10
        const stabilityProviderAccountsTwo = await operations_stability.newStabilityProvider(
            provider,
            program,
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityPoolDeposit,
        );

        // triggers one emission event, userone > 5, usertwo > 5
        const stabilityProviderAccountsThree = await operations_stability.newStabilityProvider(
            provider,
            program,
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityPoolDeposit,
        );

        const liquidationsQueue = await program.account.liquidationsQueue.fetch(stabilityPoolAccounts.liquidationsQueue);
        console.log("Liquidations queue", JSON.stringify(liquidationsQueue));

        // Harvest - any token to trigger HBB emissions
        const balanceBeforeHarvestHbbOne = await getTokenAccountBalance(program, stabilityProviderAccountsOne.stabilityProviderAccounts.hbbAta);
        const balanceBeforeHarvestHbbTwo = await getTokenAccountBalance(program, stabilityProviderAccountsTwo.stabilityProviderAccounts.hbbAta);
        const balanceBeforeHarvestHbbThree = await getTokenAccountBalance(program, stabilityProviderAccountsThree.stabilityProviderAccounts.hbbAta);

        // trigger one emission event, user1, 2, 3> 3.33 -> he harvests it all -> total user 1 = 3.33 + 5 + 10
        await operations_stability.harvestLiquidationGains(program, stabilityProviderAccountsOne.stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccountsOne.stabilityProviderAccounts);
        // trigger one emission event, user1, 2, 3> 3.33 -> but only user 2 is harvesting it = 5 + 3.33 + 3.33
        await operations_stability.harvestLiquidationGains(program, stabilityProviderAccountsTwo.stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccountsTwo.stabilityProviderAccounts);
        // trigger one emission event, user1, 2, 3> 3.33 -> but only user 3 is harvesting it = 9.99 (should be 10 really)
        await operations_stability.harvestLiquidationGains(program, stabilityProviderAccountsThree.stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccountsThree.stabilityProviderAccounts);

        const balanceAfterHarvestHbbOne = await getTokenAccountBalance(program, stabilityProviderAccountsOne.stabilityProviderAccounts.hbbAta);
        const balanceAfterHarvestHbbTwo = await getTokenAccountBalance(program, stabilityProviderAccountsTwo.stabilityProviderAccounts.hbbAta);
        const balanceAfterHarvestHbbThree = await getTokenAccountBalance(program, stabilityProviderAccountsThree.stabilityProviderAccounts.hbbAta);

        console.log(`Before Harvest One ${balanceBeforeHarvestHbbOne} After Harvest HBB ${balanceAfterHarvestHbbOne} `);
        console.log(`Before Harvest Two ${balanceBeforeHarvestHbbTwo} After Harvest HBB ${balanceAfterHarvestHbbTwo} `);
        console.log(`Before Harvest Three ${balanceBeforeHarvestHbbThree} After Harvest HBB ${balanceAfterHarvestHbbThree} `);

        // assert.strictEqual(balanceAfterHarvestHbbOne, 18.3333); // 3.33 + 5 + 10 = 18.33
        // assert.strictEqual(balanceAfterHarvestHbbTwo, 11.6666); // 5 + 3.33 + 3.33 = 11.66
        // assert.strictEqual(balanceAfterHarvestHbbThree, 10); // 9.99 (should be 10 really)

        // TODO: since we're on a real schedule now, timestamps are 0
        assert.strictEqual(balanceAfterHarvestHbbOne, 0.0); // 3.33 + 5 + 10 = 18.33
        assert.strictEqual(balanceAfterHarvestHbbTwo, 0.0); // 5 + 3.33 + 3.33 = 11.66
        assert.strictEqual(balanceAfterHarvestHbbThree, 0.0); // 9.99 (should be 10 really)

        await assertStabilityProviderBalance(provider, program, stabilityProviderAccountsOne.stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccountsOne.stabilityProviderAccounts,
            stabilityPoolDeposit, 0);
    });

    it('tests_stability_liquidate_cannot_harvest_without_clearing', async () => {

        const borrowerSolDeposit = 10;
        const borrowerFttDeposit = 12;
        const expectedLiquiationGains = 9.95; // 0.5 is fee
        const borrowerStablecoinBorrow = 300;
        const stabilityPoolDeposit = 400;

        // Set up global accouunts
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await createMarketAndStabilityPool(env);

        const pythPrices = await set_up.setUpPrices(provider, pyth, {
            solPrice: 20.0,
            ethPrice: 10.0,
            btcPrice: 10.0,
            srmPrice: 10.0,
            fttPrice: 18.0,
            rayPrice: 10.0,
        }
        );

        // we need one more, cannot liquidate the last user
        await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
            ["SOL", 1000],
        ]));

        const { borrower, borrowerAccounts } = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, borrowerStablecoinBorrow, new Map<CollateralToken, number>([
            ["SOL", borrowerSolDeposit],
            ["FTT", borrowerFttDeposit],
        ]));

        // Provide stability
        const { stabilityProvider, stabilityProviderAccounts } = await operations_stability.newStabilityProvider(
            provider,
            program,
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityPoolDeposit,
        );

        const { liquidator, liquidatorAccounts, } = await newLiquidator(provider, program, borrowingAccounts);
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            stabilityPoolDeposit, 0);

        // prices drop
        const reducedPythPrices = await set_up.setUpPrices(provider, pyth, {
            solPrice: 15.0,
            ethPrice: 10.0,
            btcPrice: 10.0,
            srmPrice: 10.0,
            fttPrice: 13.5,
            rayPrice: 10.0,
        });

        // Liquidate without clearing
        await operations_stability.tryLiquidate(program, liquidator, borrowingAccounts, stabilityPoolAccounts, borrowerAccounts, liquidatorAccounts, reducedPythPrices,
            false);

        // Harvest either token gains should fail
        await expect(operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts,
            "SOL"))
            .to.be.rejectedWith("Cannot harvest until liquidation gains are cleared");
        await expect(operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts,
            "FTT"))
            .to.be.rejectedWith("Cannot harvest until liquidation gains are cleared");

        // Provide more stability should work
        const extraStablecoinToProvide = 10;
        await instructions_borrow.airdropStablecoin(
            program,
            initialMarketOwner,
            borrowingAccounts.borrowingMarketState.publicKey,
            stabilityProviderAccounts.stablecoinAta,
            borrowingAccounts.stablecoinMint,
            utils.decimalToU64(extraStablecoinToProvide),
        );

        await instructions_stability.provideStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            stabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(extraStablecoinToProvide),
            [stabilityProvider]
        );

        // 400 + 10 - 300 * 1.005 = 108.50000000000006
        // doing -1 cause we always remove one decimal extra in the stability pool calculations
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            u64ToDecimal(decimalToU64(stabilityPoolDeposit + extraStablecoinToProvide - 300 * 1.005) - 1), 0);

        await instructions_stability.withdrawStability(
            program,
            stabilityProvider.publicKey,
            stabilityProviderAccounts.stabilityProviderState.publicKey,
            borrowingAccounts.borrowingMarketState.publicKey,
            borrowingAccounts.stabilityPoolState.publicKey,
            stabilityPoolAccounts.stabilityVaults.publicKey,
            stabilityPoolAccounts.epochToScaleToSum,
            stabilityPoolAccounts.stablecoinStabilityPoolVault,
            stabilityProviderAccounts.stablecoinAta,
            utils.decimalToU64(extraStablecoinToProvide),
            [stabilityProvider]
        );

        // doing -1 cause we always remove one decimal extra in the stability pool calculations
        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            u64ToDecimal(decimalToU64(stabilityPoolDeposit - 300 * 1.005) - 1), extraStablecoinToProvide);
    });

    it('tests_stability_liquidate_cannot_harvest_partially_clear', async () => {

        const borrowerSolDeposit = 10;
        const borrowerFttDeposit = 12;
        const expectedLiquidationGains = 9.95; // 0.5 is fee
        const borrowerStablecoinBorrow = 300;
        const stabilityPoolDeposit = 400;

        // Set up global accuunts
        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await createMarketAndStabilityPool(env);

        const pythPrices = await set_up.setUpPrices(provider, pyth, {
            solPrice: 20.0,
            ethPrice: 10.0,
            btcPrice: 10.0,
            srmPrice: 10.0,
            fttPrice: 18.0,
            rayPrice: 10.0,
        });

        // we need one more, cannot liquidate the last user
        await operations_borrowing.newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, 2000, new Map<CollateralToken, number>([
            ["SOL", 1000]
        ]));

        const { borrower, borrowerAccounts } = await newLoanee(env, borrowingAccounts, stakingPoolAccounts, pythPrices, borrowerStablecoinBorrow, new Map<CollateralToken, number>([
            ["SOL", borrowerSolDeposit],
            ["FTT", borrowerFttDeposit]
        ]));

        // Provide stability
        const { stabilityProvider, stabilityProviderAccounts, stabilityProviderInitialBalance } = await operations_stability.newStabilityProvider(
            provider,
            program,
            borrowingAccounts,
            stabilityPoolAccounts,
            stabilityPoolDeposit,
        );

        const { liquidator, liquidatorAccounts, } = await newLiquidator(provider, program, borrowingAccounts);

        await assertStabilityProviderBalance(provider, program, stabilityProvider.publicKey, borrowingAccounts, stabilityProviderAccounts,
            stabilityPoolDeposit, 0);

        // prices drop
        const reducedPythPrices = await set_up.setUpPrices(provider, pyth, {
            solPrice: 15.0,
            ethPrice: 10.0,
            btcPrice: 10.0,
            srmPrice: 10.0,
            fttPrice: 13.5,
            rayPrice: 10.0,
        });

        // Liquidate without clearing
        await operations_stability.tryLiquidate(program, liquidator, borrowingAccounts, stabilityPoolAccounts, borrowerAccounts, liquidatorAccounts, reducedPythPrices,
            false);

        // Partially clear just SOL
        await operations_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidatorAccounts,
            [liquidator],
            "SOL"
        )

        // Harvest either token gains should fail
        await expect(operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts,
            "SOL")).to.be.rejectedWith("Cannot harvest until liquidation gains are cleared")
        await expect(operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts,
            "FTT")).to.be.rejectedWith("Cannot harvest until liquidation gains are cleared")

        const liquidatorBalanceBeforeClearFtt = await getTokenAccountBalance(program, liquidatorAccounts.fttAta);

        // Clear FTT, should be the last one
        await operations_stability.clearLiquidationGains(
            program,
            liquidator.publicKey,
            borrowingAccounts,
            stabilityPoolAccounts,
            liquidatorAccounts,
            [liquidator],
            "FTT"
        );

        const liquidatorBalanceAfterClearFtt = await getTokenAccountBalance(program, liquidatorAccounts.fttAta);

        // Harvest either token gains should be no error
        await operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts,
            "SOL");
        await operations_stability.harvestLiquidationGains(program, stabilityProvider, borrowingAccounts, stabilityPoolAccounts, stabilityProviderAccounts,
            "FTT");

        console.log("liquidatorBalanceBeforeClearFtt", liquidatorBalanceBeforeClearFtt);
        console.log("liquidatorBalanceAfterClearFtt", liquidatorBalanceAfterClearFtt);

        // should be borrowerFttDeposit * 0.005
        assert.strictEqual(liquidatorBalanceBeforeClearFtt, 0);
        assert.strictEqual(liquidatorBalanceAfterClearFtt, borrowerFttDeposit * 0.005);
    });

    it('tests_stability_test_epoch_to_scale_to_sum_deserialize', async () => {

        const { borrowingAccounts, stabilityPoolAccounts, stakingPoolAccounts } = await createMarketAndStabilityPool(env);

        let account: any = await program.account.epochToScaleToSumAccount.fetch(stabilityPoolAccounts.epochToScaleToSum);
        let hmap = deserializeEpoch(account.data);
        console.log("Epoch", hmap);
    })

    async function printEpoch(acc: PublicKey) {
        const account = await program.account.epochToScaleToSumAccount.fetch(acc);
        //@ts-ignore
        let data = account.data;

        let len = data[0];
        for (let i = 0; i < len; i++) {
            console.log(`data[${i}] = ${data[i]} ${new anchor.BN(data[i])}`);
        }
    }

    function deserializeEpoch(data: any[]) {
        let hmap = [];
        let num_epochs = BigInt(data[1]);
        let current_cursor = 1;
        for (let i = 0; i < num_epochs; i++) {
            let scale = [];
            current_cursor += 1;
            let scale_length = data[current_cursor] as number;
            for (let j = 0; j < scale_length; j++) {
                let tokenMap = {
                    sol: BigInt(data[current_cursor + 1]),
                    eth: BigInt(data[current_cursor + 2]),
                    btc: BigInt(data[current_cursor + 3]),
                    srm: BigInt(data[current_cursor + 4]),
                    ray: BigInt(data[current_cursor + 5]),
                    ftt: BigInt(data[current_cursor + 6]),
                }
                scale.push(tokenMap);
                current_cursor += 6;
            }

            hmap.push(scale);
        }
        return hmap;
    }

});