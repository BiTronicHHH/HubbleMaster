import * as anchor from "@project-serum/anchor";
import * as set_up from "../src/set_up";
import { setUpProgram } from "../src/set_up";
import * as utils from "../src/utils";

import * as instructions_borrow from '../src/instructions_borrow';
import * as instructions_staking from '../src/instructions_staking';
import * as operations_borrowing from './operations_borrowing';
import * as operations_staking from './operations_staking';

import { displayBorrowingMarketState, displayStakingPoolState, displayUserStakingPoolStateAccount } from '../src/utils_display';
import { getStakingPoolState, getUserStakingStateData } from './data_provider';

import * as assert from "assert";

import { assertBorrowerBalance, assertBorrowerCollateral, assertGlobalCollateral, assertStakerBalance, assertStakingPoolBalance } from "./test_assertions";
import { sleep } from "@project-serum/common";
import { CollateralToken } from "./types";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'

chai.use(chaiAsPromised)

describe('tests_staking', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('tests_staking_staking_setup_global_accounts', async () => {
        const borrowingAccounts = await set_up.setUpBorrowingGlobalAccounts(
            provider,
            initialMarketOwner,
            program);

        const stakingAccounts = await set_up.setUpStakingPoolAccounts(
            provider,
            initialMarketOwner,
            program,
            borrowingAccounts
        );
        console.log(`Stablecoin Mint ${borrowingAccounts.stablecoinMint.toString()}`);
        console.log(`Hbb Mint ${borrowingAccounts.hbbMint.toString()}`);
        console.log(`Burning Vault Account ${borrowingAccounts.burningVault.toString()}`);
        console.log(`Collateral Vault Account ${borrowingAccounts.collateralVaultSol.toString()}`);
        console.log(`Borrowing Fees Account ${borrowingAccounts.borrowingFeesVault.toString()}`);
        console.log(`Borrowing Market State Account ${borrowingAccounts.borrowingMarketState.toString()}`);
    });

    it('tests_staking_staking_initialize_staking_pool', async () => {
        const borrowingAccounts = await set_up.setUpBorrowingGlobalAccounts(
            provider,
            initialMarketOwner,
            program);

        const stakingAccounts = await set_up.setUpStakingPoolAccounts(
            provider,
            initialMarketOwner,
            program,
            borrowingAccounts
        );

        const treasuryFeeRate = 1500;

        await instructions_borrow
            .initializeBorrowingMarket(
                program,
                initialMarketOwner,
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

        await displayBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey);
        await displayStakingPoolState(program, borrowingAccounts.stakingPoolState.publicKey);

        const stakingPoolState = await getStakingPoolState(program, borrowingAccounts.stakingPoolState.publicKey);

        assert.strictEqual(stakingPoolState.borrowingMarketState.toString(), borrowingAccounts.borrowingMarketState.publicKey.toString());
        assert.strictEqual(stakingPoolState.stakingVault.toString(), stakingAccounts.stakingVault.toString());
    });

    it('tests_staking_staking_approve_staking', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(15, "SOL"));
        await sleep(500);

        console.log("User initialized", user.publicKey.toString());

        const userStakingPoolAccounts = await set_up.setUpStakingPoolUserAccounts(provider, [user], user.publicKey, program, borrowingMarketAccounts);

        await instructions_staking.approveStakingPool(program, user.publicKey, userStakingPoolAccounts.userStakingState, borrowingMarketAccounts.stakingPoolState.publicKey, [user]);

        const userStakingState = await getUserStakingStateData(program, userStakingPoolAccounts.userStakingState.publicKey);

        assert.strictEqual(userStakingState.version, 0);
        assert.strictEqual(userStakingState.userId, 0);
        assert.strictEqual(userStakingState.rewardsTally, BigInt(0));
        assert.strictEqual(userStakingState.userStake, 0);
        assert.strictEqual(userStakingState.owner.toBase58(), user.publicKey.toBase58());
        assert.strictEqual(userStakingState.stakingPoolState.toBase58(), borrowingMarketAccounts.stakingPoolState.publicKey.toBase58());
    });

    it('tests_staking_staking_stake_hbb', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await sleep(500);


        console.log("User initialized", user.publicKey.toString());

        const userStakingPoolAccounts = await set_up.setUpStakingPoolUserAccounts(provider, [user], user.publicKey, program, borrowingMarketAccounts);

        await instructions_staking.approveStakingPool(program, user.publicKey, userStakingPoolAccounts.userStakingState, borrowingMarketAccounts.stakingPoolState.publicKey, [user]);

        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, 0, 0, 0, 0);

        let hbbToStake = utils.decimalToU64(10);

        await instructions_borrow.airdropHbb(program, initialMarketOwner, borrowingMarketAccounts.borrowingMarketState.publicKey, userStakingPoolAccounts.userHbbAta, borrowingMarketAccounts.hbbMint, hbbToStake);

        let hbbUserBalance = await provider.connection.getTokenAccountBalance(userStakingPoolAccounts.userHbbAta);
        console.log("After airdrop hbb balance", userStakingPoolAccounts.userHbbAta.toString(), hbbUserBalance.value.uiAmountString);
        console.log("HBB balance", JSON.stringify(hbbUserBalance.value));
        assert.strictEqual(Number.parseInt(hbbUserBalance.value.amount), hbbToStake);

        await instructions_staking.stake(program,
            user.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            stakingPoolAccounts.stakingVault,
            userStakingPoolAccounts.userHbbAta,
            [user],
            hbbToStake
        );

        const userStakingState = await getUserStakingStateData(program, userStakingPoolAccounts.userStakingState.publicKey);
        const stakingPoolState = await getStakingPoolState(program, borrowingMarketAccounts.stakingPoolState.publicKey);

        assert.strictEqual(userStakingState.rewardsTally, BigInt(0));
        assert.strictEqual(userStakingState.userStake, hbbToStake, 'HBB USER BALANCE');

        assert.strictEqual(stakingPoolState.totalStake, hbbToStake, 'HBB TO STAKE');

        let hbbUserBalanceAfter = await provider.connection.getTokenAccountBalance(userStakingPoolAccounts.userStablecoinAta);
        assert.strictEqual(Number.parseInt(hbbUserBalanceAfter.value.amount), 0, 'HBB USER BALANCE AFTER');

        let stakingPoolBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.stakingVault);
        assert.strictEqual(Number.parseInt(stakingPoolBalance.value.amount), hbbToStake, 'STAKING VAULT BALANCE');

    });

    it('tests_staking_when_staking_not_approved_then_error', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await sleep(500);


        console.log("User initialized", user.publicKey.toString());

        const userStakingPoolAccounts = await set_up.setUpStakingPoolUserAccounts(provider, [user], user.publicKey, program, borrowingMarketAccounts);

        let hbbToStake = utils.decimalToU64(10);

        await instructions_borrow.airdropHbb(program, initialMarketOwner, borrowingMarketAccounts.borrowingMarketState.publicKey, userStakingPoolAccounts.userHbbAta, borrowingMarketAccounts.hbbMint, hbbToStake);

        let hbbUserBalance = await provider.connection.getTokenAccountBalance(userStakingPoolAccounts.userHbbAta);
        console.log("After airdrop hbb balance", userStakingPoolAccounts.userHbbAta.toString(), hbbUserBalance.value.uiAmountString);
        console.log("HBB balance", JSON.stringify(hbbUserBalance.value));
        assert.strictEqual(Number.parseInt(hbbUserBalance.value.amount), hbbToStake);
        await expect(instructions_staking.stake(
            program,
            user.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            stakingPoolAccounts.stakingVault, userStakingPoolAccounts.userHbbAta,
            [user],
            hbbToStake
        )).to.be.rejected;

    });

    it('tests_staking_staking_withdraw_stake', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        let hbbToStake = utils.decimalToU64(10);

        const { user, userStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        await instructions_staking.unstake(
            program,
            user.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            userStakingPoolAccounts.userHbbAta,
            userStakingPoolAccounts.userStablecoinAta,
            stakingPoolAccounts.stakingVault,
            borrowingMarketAccounts.borrowingFeesVault,
            [user],
            hbbToStake);

        await assertStakerBalance(provider, program, user.publicKey, borrowingMarketAccounts, hbbToStake, 0);
    });

    it('tests_staking_when_staking_zero_error', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await sleep(500);

        console.log("User initialized", user.publicKey.toString());

        const userStakingPoolAccounts = await set_up.setUpStakingPoolUserAccounts(provider, [user], user.publicKey, program, borrowingMarketAccounts);

        await instructions_staking.approveStakingPool(program, user.publicKey, userStakingPoolAccounts.userStakingState, borrowingMarketAccounts.stakingPoolState.publicKey, [user]);

        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, 0, 0, 0, 0);

        await expect(instructions_staking.stake(program,
            user.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            stakingPoolAccounts.stakingVault,
            userStakingPoolAccounts.userHbbAta,
            [user],
            0
        )).to.be.rejected;
    });

    it('tests_staking_unstaking_too_much_single', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        let hbbToStake = utils.decimalToU64(10);

        const { user, userStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        await instructions_staking.unstake(
            program,
            user.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            userStakingPoolAccounts.userHbbAta,
            userStakingPoolAccounts.userStablecoinAta,
            stakingPoolAccounts.stakingVault,
            borrowingMarketAccounts.borrowingFeesVault,
            [user],
            hbbToStake + 2);

        await assertStakerBalance(provider, program, user.publicKey, borrowingMarketAccounts, hbbToStake, 0);

    });

    it('tests_staking_unstaking_too_much_multiple', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        let hbbToStake = utils.decimalToU64(10);

        const { user: alice, userStakingPoolAccounts: aliceStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);
        const { user: bob, userStakingPoolAccounts: bobStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        await instructions_staking.unstake(
            program,
            alice.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            aliceStakingPoolAccounts.userStakingState.publicKey,
            aliceStakingPoolAccounts.userHbbAta,
            aliceStakingPoolAccounts.userStablecoinAta,
            stakingPoolAccounts.stakingVault,
            borrowingMarketAccounts.borrowingFeesVault,
            [alice],
            hbbToStake + 2);

        await assertStakerBalance(provider, program, alice.publicKey, borrowingMarketAccounts, hbbToStake, 0);
        await assertStakerBalance(provider, program, bob.publicKey, borrowingMarketAccounts, 0, 0);
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake, 0, 0, 0);
    });

    it('tests_staking_staking_stake_withdraw_multiple', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        let hbbToStake = utils.decimalToU64(10);

        let i = 0;

        while (i < 20) {
            const { user, userStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

            await instructions_staking.unstake(
                program,
                user.publicKey,
                borrowingMarketAccounts.borrowingMarketState.publicKey,
                borrowingMarketAccounts.borrowingVaults.publicKey,
                borrowingMarketAccounts.stakingPoolState.publicKey,
                userStakingPoolAccounts.userStakingState.publicKey,
                userStakingPoolAccounts.userHbbAta,
                userStakingPoolAccounts.userStablecoinAta,
                stakingPoolAccounts.stakingVault,
                borrowingMarketAccounts.borrowingFeesVault,
                [user],
                hbbToStake + 2);

            await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, 0, 0, 0, 0);
            await assertStakerBalance(provider, program, user.publicKey, borrowingMarketAccounts, hbbToStake, 0);

            i++;
        }

    });

    it('tests_staking_staking_drop_rewards', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        let hbbToStake = utils.decimalToU64(10);

        const { user, userStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        const { borrower, borrowerAccounts, borrowerInitialBalance } = await operations_borrowing.newBorrowingUser(env, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", 10]
        ]));

        const depositSol = 5;
        await operations_borrowing.depositCollateral(provider, program, depositSol, borrower, borrowerAccounts, borrowingMarketAccounts);
        await sleep(1000);
        await assertGlobalCollateral(
            program,
            provider,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),
            new Map<CollateralToken, number>([["SOL", 5]]),
        );

        // borrow stable
        const borrowStablecoin = 300;
        await operations_borrowing.borrow(provider, program, borrowStablecoin, borrower, borrowerAccounts, borrowingMarketAccounts, stakingPoolAccounts, pythPrices);

        // assert balances
        await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, 301.5, borrowerInitialBalance - depositSol, 300);
        await assertBorrowerCollateral(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", depositSol]
        ]))

        let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);
        let treasuryVaultBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.treasuryVault);

        assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), utils.decimalToU64(1.275), 'BORROWING FEES VAULT BALANCE');
        assert.strictEqual(Number.parseInt(treasuryVaultBalance.value.amount), utils.decimalToU64(0.225), 'TREASURY VAULT BALANCE');
        assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake, 1.275, 1.275, 1.275 / 10);
    });

    // it('tests_staking_staking_stake_u64_max', async () => {
    //     const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(
    //         provider,
    //         program,
    //         initialMarketOwner
    //     );

    //     let hbbToStake = 1;

    //     const { user, userStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

    //     let up_to = 10; // 10 ** 9
    //     for (let i = 0; i < up_to; i++) {
    //         console.log("A");

    //         const { borrower, borrowerAccounts } = await operations_borrowing.newBorrowingUser(provider, program, 10000, borrowingMarketAccounts);

    //         const depositSol = 9000;
    //         await operations_borrowing.depositCollateral(provider, program, depositSol, borrower, borrowerAccounts, borrowingMarketAccounts);
    //         await sleep(500);
    //         await assertGlobalCollateral(program, provider, borrowingMarketAccounts.borrowingMarketState.publicKey, 9000 * (i + 1));

    //         // borrow stable
    //         const borrowStablecoin = 45000;
    //         await operations_borrowing.borrow(provider, program, borrowStablecoin, borrower, borrowerAccounts, borrowingMarketAccounts);

    //         // assert balances
    //         await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, depositSol, 45225, 45000); // 45000 * 0.005 + 45000

    //         let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);

    //         assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), (i + 1) * utils.decimalToU64(225), 'Assert Borrowing Fees Vault Balance');
    //         console.log("Borrowing Fees Vault", Number.parseInt(borrowingFeesVaultBalance.value.amount));
    //         const stakingPoolState = await getStakingPoolState(program, borrowingMarketAccounts.stakingPoolState.publicKey);
    //         console.log("Reward Per Token", stakingPoolState.rewardPerToken);
    //         assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake, (i + 1) * utils.decimalToU64(225), (i + 1) * utils.decimalToU64(225), (i + 1) * 225);
    //     }


    // });

    it('tests_staking_staking_harvest_reward', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        let hbbToStake = utils.decimalToU64(10);

        const { user, userStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        const { borrower, borrowerAccounts, borrowerInitialBalance } = await operations_borrowing.newBorrowingUser(env, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", 15]
        ]));

        const depositSol = 10;
        await operations_borrowing.depositCollateral(provider, program, depositSol, borrower, borrowerAccounts, borrowingMarketAccounts);
        await sleep(1000);
        await assertGlobalCollateral(
            program,
            provider,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),
            new Map<CollateralToken, number>([["SOL", 10]]),
        );

        // borrow stable
        const borrowStablecoin = 300;
        await operations_borrowing.borrow(provider, program, borrowStablecoin, borrower, borrowerAccounts, borrowingMarketAccounts, stakingPoolAccounts, pythPrices);

        // assert balances
        await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, 301.5, borrowerInitialBalance - depositSol, 300);
        await assertBorrowerCollateral(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", depositSol]
        ]))

        let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);
        let treasuryVaultBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.treasuryVault);

        assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), utils.decimalToU64(1.275), 'BORROWING FEES VAULT BALANCE');
        assert.strictEqual(Number.parseInt(treasuryVaultBalance.value.amount), utils.decimalToU64(0.225), 'TREASURY VAULT BALANCE');
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake, 1.275, 1.275, 1.275 / 10);

        await instructions_staking.harvestReward(program, user.publicKey, borrowingMarketAccounts.borrowingMarketState.publicKey, borrowingMarketAccounts.borrowingVaults.publicKey, borrowingMarketAccounts.stakingPoolState.publicKey, userStakingPoolAccounts.userStakingState.publicKey, userStakingPoolAccounts.userStablecoinAta, borrowingMarketAccounts.borrowingFeesVault, [user]);

        await displayUserStakingPoolStateAccount(program, userStakingPoolAccounts.userStakingState.publicKey);
        const userStakingState = await getUserStakingStateData(program, userStakingPoolAccounts.userStakingState.publicKey);

        assert.strictEqual(userStakingState.rewardsTally, BigInt(utils.decimalToU64(10 * 1.275 / 10) * 1000000000000), "Rewards Tally Assertion");

        await assertStakerBalance(provider, program, user.publicKey, borrowingMarketAccounts, 0, 1.275);
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake, 1.275, 0, 1.275 / 10);

    });

    it('tests_staking_when_no_stake_harvest_error', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await sleep(500);


        console.log("User initialized", user.publicKey.toString());

        const userStakingPoolAccounts = await set_up.setUpStakingPoolUserAccounts(provider, [user], user.publicKey, program, borrowingMarketAccounts);

        await instructions_staking.approveStakingPool(program, user.publicKey, userStakingPoolAccounts.userStakingState, borrowingMarketAccounts.stakingPoolState.publicKey, [user]);

        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, 0, 0, 0, 0);

        let hbbToStake = utils.decimalToU64(10);

        await instructions_borrow.airdropHbb(program, initialMarketOwner, borrowingMarketAccounts.borrowingMarketState.publicKey, userStakingPoolAccounts.userHbbAta, borrowingMarketAccounts.hbbMint, hbbToStake);

        let hbbUserBalance = await provider.connection.getTokenAccountBalance(userStakingPoolAccounts.userHbbAta);
        console.log("After airdrop hbb balance", userStakingPoolAccounts.userHbbAta.toString(), hbbUserBalance.value.uiAmountString);
        console.log("HBB balance", JSON.stringify(hbbUserBalance.value));
        assert.strictEqual(Number.parseInt(hbbUserBalance.value.amount), hbbToStake);
        await expect(instructions_staking.harvestReward(program,
            user.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            userStakingPoolAccounts.userStablecoinAta,
            borrowingMarketAccounts.borrowingFeesVault,
            [user]
        )).to.be.rejected;
    });

    it('tests_staking_when_no_reward_harvest_error', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        const hbbToStake = utils.decimalToU64(10);
        const { user, userStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);
        await expect(instructions_staking.harvestReward(program,
            user.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            userStakingPoolAccounts.userStablecoinAta,
            borrowingMarketAccounts.borrowingFeesVault,
            [user]
        )).to.be.rejected;
    });

    it('tests_staking_staking_unstake_and_reward', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        let hbbToStake = utils.decimalToU64(10);

        const { user, userStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        const { borrower, borrowerAccounts, borrowerInitialBalance } = await operations_borrowing.newBorrowingUser(env, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", 15]
        ]));

        const depositSol = 10;
        await operations_borrowing.depositCollateral(provider, program, depositSol, borrower, borrowerAccounts, borrowingMarketAccounts);
        await sleep(1000);
        await assertGlobalCollateral(
            program, provider,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),
            new Map<CollateralToken, number>([["SOL", 10]]),
        );

        // borrow stable
        const borrowStablecoin = 300;
        await operations_borrowing.borrow(provider, program, borrowStablecoin, borrower, borrowerAccounts, borrowingMarketAccounts, stakingPoolAccounts, pythPrices);

        // assert balances
        await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, 301.5, borrowerInitialBalance - depositSol, 300);
        await assertBorrowerCollateral(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", depositSol]
        ]))

        let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);
        let treasuryVaultBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.treasuryVault);

        assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), utils.decimalToU64(1.275), 'BORROWING FEES VAULT BALANCE');
        assert.strictEqual(Number.parseInt(treasuryVaultBalance.value.amount), utils.decimalToU64(0.225), 'TREASURY VAULT BALANCE');
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake, 1.275, 1.275, 1.275 / 10);

        await instructions_staking.unstake(program, user.publicKey, borrowingMarketAccounts.borrowingMarketState.publicKey, borrowingMarketAccounts.borrowingVaults.publicKey, borrowingMarketAccounts.stakingPoolState.publicKey, userStakingPoolAccounts.userStakingState.publicKey, userStakingPoolAccounts.userHbbAta, userStakingPoolAccounts.userStablecoinAta, stakingPoolAccounts.stakingVault, borrowingMarketAccounts.borrowingFeesVault, [user], hbbToStake);

        await assertStakerBalance(provider, program, user.publicKey, borrowingMarketAccounts, hbbToStake, 1.275);
        {
            let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);

            assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), utils.decimalToU64(0), 'Borrowing Fees Vault Balance After Unstaking');
        }

    });

    it('tests_staking_double_staking_harvesting', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(5, "SOL"));
        await sleep(500);


        console.log("User initialized", user.publicKey.toString());

        const userStakingPoolAccounts = await set_up.setUpStakingPoolUserAccounts(provider, [user], user.publicKey, program, borrowingMarketAccounts);

        await instructions_staking.approveStakingPool(program, user.publicKey, userStakingPoolAccounts.userStakingState, borrowingMarketAccounts.stakingPoolState.publicKey, [user]);

        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, 0, 0, 0, 0);

        let hbbToStake = utils.decimalToU64(20);

        await instructions_borrow.airdropHbb(program, initialMarketOwner, borrowingMarketAccounts.borrowingMarketState.publicKey, userStakingPoolAccounts.userHbbAta, borrowingMarketAccounts.hbbMint, hbbToStake);

        let hbbUserBalance = await provider.connection.getTokenAccountBalance(userStakingPoolAccounts.userHbbAta);
        console.log("After airdrop hbb balance", userStakingPoolAccounts.userHbbAta.toString(), hbbUserBalance.value.uiAmountString);
        console.log("HBB balance", JSON.stringify(hbbUserBalance.value));
        assert.strictEqual(Number.parseInt(hbbUserBalance.value.amount), hbbToStake);

        await instructions_staking.stake(program,
            user.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            stakingPoolAccounts.stakingVault,
            userStakingPoolAccounts.userHbbAta,
            [user],
            hbbToStake / 2);

        let hbbUserBalanceAfter = await provider.connection.getTokenAccountBalance(userStakingPoolAccounts.userStablecoinAta);
        assert.strictEqual(Number.parseInt(hbbUserBalanceAfter.value.amount), 0, 'HBB USER BALANCE AFTER');

        let stakingPoolBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.stakingVault);
        assert.strictEqual(Number.parseInt(stakingPoolBalance.value.amount), hbbToStake / 2, 'STAKING VAULT BALANCE');

        const { borrower, borrowerAccounts, borrowerInitialBalance } = await operations_borrowing.newBorrowingUser(env, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", 15]
        ]));

        const depositSol = 10;
        await operations_borrowing.depositCollateral(provider, program, depositSol, borrower, borrowerAccounts, borrowingMarketAccounts);
        await sleep(1000);
        await assertGlobalCollateral(
            program,
            provider,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),
            new Map<CollateralToken, number>([["SOL", 10]]),
        );

        // borrow stable
        const borrowStablecoin = 300;
        await operations_borrowing.borrow(provider, program, borrowStablecoin, borrower, borrowerAccounts, borrowingMarketAccounts, stakingPoolAccounts, pythPrices);

        // assert balances
        await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, 301.5, borrowerInitialBalance - depositSol, 300);
        await assertBorrowerCollateral(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", depositSol]
        ]))

        let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);
        let treasuryVaultBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.treasuryVault);

        assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), utils.decimalToU64(1.275), 'BORROWING FEES VAULT BALANCE');
        assert.strictEqual(Number.parseInt(treasuryVaultBalance.value.amount), utils.decimalToU64(0.225), 'TREASURY VAULT BALANCE');
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake / 2, 1.275, 1.275, 1.275 / 10);

        await instructions_staking.stake(program,
            user.publicKey,
            userStakingPoolAccounts.userStakingState.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            stakingPoolAccounts.stakingVault,
            userStakingPoolAccounts.userHbbAta,
            [user],
            hbbToStake / 2
        );

        await instructions_staking.harvestReward(program, user.publicKey, borrowingMarketAccounts.borrowingMarketState.publicKey, borrowingMarketAccounts.borrowingVaults.publicKey, borrowingMarketAccounts.stakingPoolState.publicKey, userStakingPoolAccounts.userStakingState.publicKey, userStakingPoolAccounts.userStablecoinAta, borrowingMarketAccounts.borrowingFeesVault, [user]);

        await displayUserStakingPoolStateAccount(program, userStakingPoolAccounts.userStakingState.publicKey);
        const userStakingState = await getUserStakingStateData(program, userStakingPoolAccounts.userStakingState.publicKey);

        assert.strictEqual(userStakingState.rewardsTally, BigInt(utils.decimalToU64(20 * 1.275 / 10) * 1000000000000), "Rewards Tally Assertion");

        await assertStakerBalance(provider, program, user.publicKey, borrowingMarketAccounts, 0, 1.275);
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake, 1.275, 0, 1.275 / 10);

    });

    it('tests_staking_double_staking_harvesting_different', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);
        let hbbToStake = utils.decimalToU64(30);

        const { user: userOne, userStakingPoolAccounts: userOneStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake / 3);
        const { user: userTwo, userStakingPoolAccounts: userTwoStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake / 3);

        const { borrower, borrowerAccounts, borrowerInitialBalance } = await operations_borrowing.newBorrowingUser(env, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", 15]
        ]));

        const depositSol = 10;
        await operations_borrowing.depositCollateral(provider, program, depositSol, borrower, borrowerAccounts, borrowingMarketAccounts);
        await sleep(1000);
        await assertGlobalCollateral(
            program,
            provider,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),
            new Map<CollateralToken, number>([["SOL", 10]]),
        );

        // borrow stable
        const borrowStablecoin = 300;
        await operations_borrowing.borrow(provider, program, borrowStablecoin, borrower, borrowerAccounts, borrowingMarketAccounts, stakingPoolAccounts, pythPrices);

        // assert balances
        await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, 301.5, borrowerInitialBalance - depositSol, 300);
        await assertBorrowerCollateral(provider, program, borrower, borrowerAccounts, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", depositSol]
        ]))

        let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);
        let treasuryVaultBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.treasuryVault);

        assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), utils.decimalToU64(1.275), 'BORROWING FEES VAULT BALANCE');
        assert.strictEqual(Number.parseInt(treasuryVaultBalance.value.amount), utils.decimalToU64(0.225), 'TREASURY VAULT BALANCE');
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, 2 * hbbToStake / 3, 1.275, 1.275, 1.275 / 20);

        await instructions_staking.harvestReward(program, userOne.publicKey, borrowingMarketAccounts.borrowingMarketState.publicKey, borrowingMarketAccounts.borrowingVaults.publicKey, borrowingMarketAccounts.stakingPoolState.publicKey, userOneStakingPoolAccounts.userStakingState.publicKey, userOneStakingPoolAccounts.userStablecoinAta, borrowingMarketAccounts.borrowingFeesVault, [userOne]);
        await instructions_staking.harvestReward(program, userTwo.publicKey, borrowingMarketAccounts.borrowingMarketState.publicKey, borrowingMarketAccounts.borrowingVaults.publicKey, borrowingMarketAccounts.stakingPoolState.publicKey, userTwoStakingPoolAccounts.userStakingState.publicKey, userTwoStakingPoolAccounts.userStablecoinAta, borrowingMarketAccounts.borrowingFeesVault, [userTwo]);

        await displayUserStakingPoolStateAccount(program, userOneStakingPoolAccounts.userStakingState.publicKey);
        const userOneStakingState = await getUserStakingStateData(program, userOneStakingPoolAccounts.userStakingState.publicKey);


        await assertStakerBalance(provider, program, userOne.publicKey, borrowingMarketAccounts, 0, 1.275 / 2);
        await assertStakerBalance(provider, program, userTwo.publicKey, borrowingMarketAccounts, 0, 1.275 / 2);

        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake * 2 / 3, 1.275, 0, 1.275 / 20);
    });

    it('tests_staking_double_staking_async_harvesting_different', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        let hbbToStake = utils.decimalToU64(10);

        const { user: userOne, userStakingPoolAccounts: userOneStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        const { borrower, borrowerAccounts } = await operations_borrowing.newBorrowingUser(env, borrowingMarketAccounts, new Map<CollateralToken, number>([
            ["SOL", 15]
        ]));

        const depositSol = 10;
        const borrowStablecoin = 300;

        await operations_staking.triggerFees(env, borrowingMarketAccounts, stakingPoolAccounts, depositSol, borrowStablecoin, pythPrices);

        const { user: userTwo, userStakingPoolAccounts: userTwoStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);
        let treasuryVaultBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.treasuryVault);

        assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), utils.decimalToU64(1.275), 'BORROWING FEES VAULT BALANCE');
        assert.strictEqual(Number.parseInt(treasuryVaultBalance.value.amount), utils.decimalToU64(0.225), 'TREASURY VAULT BALANCE');
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, 2 * hbbToStake, 1.275, 1.275, 1.275 / 10);

        await instructions_staking.harvestReward(program, userOne.publicKey, borrowingMarketAccounts.borrowingMarketState.publicKey, borrowingMarketAccounts.borrowingVaults.publicKey, borrowingMarketAccounts.stakingPoolState.publicKey, userOneStakingPoolAccounts.userStakingState.publicKey, userOneStakingPoolAccounts.userStablecoinAta, borrowingMarketAccounts.borrowingFeesVault, [userOne]);
        await expect(instructions_staking.harvestReward(program,
            userTwo.publicKey,
            borrowingMarketAccounts.borrowingMarketState.publicKey,
            borrowingMarketAccounts.borrowingVaults.publicKey,
            borrowingMarketAccounts.stakingPoolState.publicKey,
            userTwoStakingPoolAccounts.userStakingState.publicKey,
            userTwoStakingPoolAccounts.userStablecoinAta,
            borrowingMarketAccounts.borrowingFeesVault,
            [userTwo]
        )).to.be.rejected;

        await displayUserStakingPoolStateAccount(program, userOneStakingPoolAccounts.userStakingState.publicKey);
        const userOneStakingState = await getUserStakingStateData(program, userOneStakingPoolAccounts.userStakingState.publicKey);

        assert.strictEqual(userOneStakingState.rewardsTally, BigInt(utils.decimalToU64(10 * 1.275 / 10) * 1000000000000), "Rewards Tally Assertion");

        await assertStakerBalance(provider, program, userOne.publicKey, borrowingMarketAccounts, 0, 1.275);
        await assertStakerBalance(provider, program, userTwo.publicKey, borrowingMarketAccounts, 0, 0);

        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake * 2, 1.275, 0, 1.275 / 10);
    })

    it('tests_staking_double_staking_async_harvesting_both', async () => {
        const { stakingPoolAccounts, borrowingMarketAccounts } = await operations_staking.initalizeMarketAndStakingPool(env);

        let hbbToStake = utils.decimalToU64(10);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const { user: userOne, userStakingPoolAccounts: userOneStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        // const { borrower: alice, borrowerAccounts: aliceAccounts } = await operations_borrowing.newBorrowingUser(provider, program, 15, borrowingMarketAccounts);

        const depositSol = 40;
        const borrowStablecoin = 200;

        await operations_staking.triggerFees(env, borrowingMarketAccounts, stakingPoolAccounts, depositSol, borrowStablecoin, pythPrices);

        const { user: userTwo, userStakingPoolAccounts: userTwoStakingPoolAccounts } = await operations_staking.newStakingPoolUser(provider, program, initialMarketOwner, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake);

        // const { borrower: bob, borrowerAccounts: bobAccounts } = await operations_borrowing.newBorrowingUser(provider, program, 15, borrowingMarketAccounts);

        await operations_staking.triggerFees(env, borrowingMarketAccounts, stakingPoolAccounts, depositSol, borrowStablecoin, pythPrices);


        let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);
        let treasuryVaultBalance = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.treasuryVault);

        assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), utils.decimalToU64(1.7), 'BORROWING FEES VAULT BALANCE');
        assert.strictEqual(Number.parseInt(treasuryVaultBalance.value.amount), utils.decimalToU64(0.3), 'TREASURY VAULT BALANCE');
        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, 2 * hbbToStake, 1.7, 1.7, 0.1275);

        await instructions_staking.harvestReward(program, userOne.publicKey, borrowingMarketAccounts.borrowingMarketState.publicKey, borrowingMarketAccounts.borrowingVaults.publicKey, borrowingMarketAccounts.stakingPoolState.publicKey, userOneStakingPoolAccounts.userStakingState.publicKey, userOneStakingPoolAccounts.userStablecoinAta, borrowingMarketAccounts.borrowingFeesVault, [userOne]);
        await instructions_staking.harvestReward(program, userTwo.publicKey, borrowingMarketAccounts.borrowingMarketState.publicKey, borrowingMarketAccounts.borrowingVaults.publicKey, borrowingMarketAccounts.stakingPoolState.publicKey, userTwoStakingPoolAccounts.userStakingState.publicKey, userTwoStakingPoolAccounts.userStablecoinAta, borrowingMarketAccounts.borrowingFeesVault, [userTwo]);

        await displayUserStakingPoolStateAccount(program, userOneStakingPoolAccounts.userStakingState.publicKey);
        const userOneStakingState = await getUserStakingStateData(program, userOneStakingPoolAccounts.userStakingState.publicKey);

        assert.strictEqual(userOneStakingState.rewardsTally, BigInt(utils.decimalToU64(10 * 0.1275) * 1000000000000), "Rewards Tally Assertion");

        await assertStakerBalance(provider, program, userOne.publicKey, borrowingMarketAccounts, 0, 1.5);

        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake * 2, 1.7, 0, 0.1275);

        await assertStakerBalance(provider, program, userTwo.publicKey, borrowingMarketAccounts, 0, 0.25);

        await assertStakingPoolBalance(provider, program, borrowingMarketAccounts, stakingPoolAccounts, hbbToStake * 2, 1.7, 0, 0.1275);

        {
            let borrowingFeesVaultBalance = await provider.connection.getTokenAccountBalance(borrowingMarketAccounts.borrowingFeesVault);

            assert.strictEqual(Number.parseInt(borrowingFeesVaultBalance.value.amount), 0, 'BORROWING FEES VAULT BALANCE');
        }
    })
})
