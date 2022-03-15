import * as anchor from '@project-serum/anchor';
import * as set_up from '../src/set_up';
import * as operations_borrowing from "./operations_borrowing";
import * as instructions_borrow from '../src/instructions_borrow';
import * as operations_staking from './operations_staking';
import * as utils from "../src/utils";
import * as assert from "assert";
import { getBorrowingMarketState, getBorrowingVaults, getGlobalConfig, getCollateralVaultBalance, getTokenAccountBalance, getUserMetadata } from './data_provider';
import { displayBorrowingMarketState, displayBorrowingVaults, displayTrove, displayUserMetadata } from '../src/utils_display';
import { newBorrowingUser } from './operations_borrowing';
import { sleep } from '@project-serum/common';
import { assertGlobalCollateral, assertGlobalDebt, assertBorrowerBalance, assertBorrowerCollateral, assertRedemptionsQueueOrderFilled } from './test_assertions';
import { CollateralToken } from "./types";
import { Keypair, PublicKey } from "@solana/web3.js";
import { setUpProgram } from "../src/set_up";

describe('tests_borrowing', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('tests_borrowing_basic', async () => {
        console.log('Your transaction signature');
    });

    it('tests_borrowing_keypair', async () => {
        let x: Uint8Array = Uint8Array.from([
            241, 101, 13, 165, 53, 150, 114, 216, 162, 246, 157, 94, 156, 209, 145, 37,
            186, 13, 219, 120, 66, 196, 128, 253, 177, 46, 0, 70, 68, 211, 238, 83, 155,
            17, 157, 105, 115, 161, 0, 60, 146, 250, 19, 171, 63, 222, 211, 135, 37, 102,
            222, 216, 142, 131, 67, 196, 185, 182, 202, 219, 55, 24, 135, 90
        ]);
        let xpk = Keypair.fromSecretKey(x);
        console.log(xpk.publicKey.toString());

        let y: Uint8Array = Uint8Array.from([
            227, 208, 96, 209, 94, 248, 249, 228, 152, 203, 169, 223, 89, 152, 61, 189, 74, 231, 111,
            238, 6, 208, 226, 251, 15, 31, 44, 191, 13, 244, 121, 222, 16, 249, 186, 206, 236, 146, 48,
            246, 2, 163, 91, 119, 79, 228, 118, 207, 67, 138, 98, 53, 182, 219, 33, 68, 109, 246, 221,
            6, 49, 8, 46, 231]);
        let ypk = Keypair.fromSecretKey(y);
        console.log(ypk.publicKey.toString());
    });

    it('tests_borrowing_airdrop', async () => {
        const user = anchor.web3.Keypair.generate();
        // 10 SOL
        await provider.connection.requestAirdrop(user.publicKey, 10000000000);
        console.log(`User ${user.publicKey}`);
    });

    it('tests_borrowing_setup_global_accounts', async () => {
        const accounts = await set_up.setUpBorrowingGlobalAccounts(
            provider,
            initialMarketOwner,
            program);

        console.log(`Stablecoin Mint ${accounts.stablecoinMint.toString()}`);
        console.log(`Burning Vault Account ${accounts.burningVault.toString()}`);
        console.log(`Collateral Vault Account collateralVaultSol ${accounts.collateralVaultSol.toString()}`);
        console.log(`Collateral Vault Account collateralVaultSrm ${accounts.collateralVaultSrm.toString()}`);
        console.log(`Collateral Vault Account collateralVaultEth ${accounts.collateralVaultEth.toString()}`);
        console.log(`Collateral Vault Account collateralVaultBtc ${accounts.collateralVaultBtc.toString()}`);
        console.log(`Collateral Vault Account collateralVaultRay ${accounts.collateralVaultRay.toString()}`);
        console.log(`Collateral Vault Account collateralVaultFtt ${accounts.collateralVaultFtt.toString()}`);
        console.log(`Borrowing Fees Account ${accounts.borrowingFeesVault.toString()}`);
        console.log(`Borrowing Market State Account ${accounts.borrowingMarketState.toString()}`);
    });

    it('tests_borrowing_mint_some_tokens', async () => {
        const accounts = await set_up.setUpBorrowingGlobalAccounts(
            provider,
            initialMarketOwner,
            program);

        console.log(`Collateral Vault Account collateralVaultSol ${accounts.collateralVaultSol.toString()}`);
        console.log(`Collateral Vault Account collateralVaultSrm ${accounts.collateralVaultSrm.toString()}`);
        console.log(`Collateral Vault Account collateralVaultEth ${accounts.collateralVaultEth.toString()}`);
        console.log(`Collateral Vault Account collateralVaultBtc ${accounts.collateralVaultBtc.toString()}`);
        console.log(`Collateral Vault Account collateralVaultRay ${accounts.collateralVaultRay.toString()}`);
        console.log(`Collateral Vault Account collateralVaultFtt ${accounts.collateralVaultFtt.toString()}`);

        console.log(`Mint Srm ${accounts.srmMint.toString()}`);
        console.log(`Mint Eth ${accounts.ethMint.toString()}`);
        console.log(`Mint Btc ${accounts.btcMint.toString()}`);
        console.log(`Mint Ray ${accounts.rayMint.toString()}`);
        console.log(`Mint Ftt ${accounts.fttMint.toString()}`);

        await utils.mintTo(provider, accounts.btcMint, accounts.collateralVaultBtc, utils.decimalToU64(100.0));

    });

    it.skip('get_all_troves', async () => {
        // Get all user troves
        let allTroves = await program.account.userMetadata.all();
        console.log(`All Troves ${JSON.stringify(allTroves.length)}`);

        // Get trove of given user
        let user = new PublicKey("9yHTd1Hn9QaaiyNuxEpxEAFa3t4zdfT1FSFBW9SwWzYZ");
        let userTrove = allTroves.filter((acc) => {
            return acc.account.owner.toString() == user.toString();
        })[0];
        console.log(`User trove is ${JSON.stringify(userTrove)}`);

        let userId = userTrove.account.userId;
        console.log(`User id is ${JSON.stringify(userId)}`);

        // Get user debt
        let borrowingMarketStatePK = new PublicKey("BySSNCKg386Y6nDHwwyjgzcLJ2WBkJCwgboXLT81ryCM");
        let userPositionsPK = new PublicKey("D1owL3jAKxb6G4LDEFr7VCZBoxyxFDFppWZZszQ4VTB6");

        let borrowingMarketState = await program.account.borrowingMarketState.fetch(borrowingMarketStatePK);
        let positions = await program.account.userPositions.fetch(userPositionsPK);
        // console.log(`User Positions ${JSON.stringify(positions)}`);

        // @ts-ignore
        let userPosition = positions.positions[userId];
        console.log(`User Position ${JSON.stringify(userPosition)}`);

        // Get user deposits
        let depositedSol = utils.lamportsToColl(userPosition.depositedCollateral.sol, "SOL");
        let depositedEth = userPosition.depositedCollateral.eth;
        let borrowedUsd = utils.u64ToDecimal(userPosition.borrowedStablecoin.toNumber());

        let solPrice = 40.0;
        let collRatio = solPrice * depositedSol / borrowedUsd;
        console.log(`User Sol ${depositedSol} Eth ${depositedEth} Debt ${borrowedUsd} Coll Ratio ${collRatio}`);

        // Can liquidate?
        console.log(`Can liquidate ${collRatio < 1.1}`);
    });

    it('tests_borrowing_initialize_market', async () => {
        console.log("Provider", provider.wallet.publicKey.toString());
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

        console.log('Initialized market');

        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        await displayBorrowingVaults(program, borrowingGlobalAccounts.borrowingVaults.publicKey);

        const borrowingMarketState = await getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        assert.strictEqual(borrowingMarketState.redemptionsQueue.toString(), borrowingGlobalAccounts.redemptionsQueue.toString());
        assert.strictEqual(borrowingMarketState.numUsers, 0);
        assert.strictEqual(borrowingMarketState.stablecoinBorrowed, 0);

        const borrowingVaults = await getBorrowingVaults(program, borrowingGlobalAccounts.borrowingVaults.publicKey);
        assert.strictEqual(borrowingVaults.borrowingMarketState.toString(), borrowingGlobalAccounts.borrowingMarketState.publicKey.toString());
        assert.strictEqual(borrowingVaults.borrowingFeesVault.toString(), borrowingGlobalAccounts.borrowingFeesVault.toString());
        assert.strictEqual(borrowingVaults.burningVault.toString(), borrowingGlobalAccounts.burningVault.toString());
        assert.strictEqual(borrowingVaults.collateralVaultSol.toString(), borrowingGlobalAccounts.collateralVaultSol.toString());
        assert.strictEqual(borrowingVaults.collateralVaultSrm.toString(), borrowingGlobalAccounts.collateralVaultSrm.toString());
        assert.strictEqual(borrowingVaults.collateralVaultEth.toString(), borrowingGlobalAccounts.collateralVaultEth.toString());
        assert.strictEqual(borrowingVaults.collateralVaultBtc.toString(), borrowingGlobalAccounts.collateralVaultBtc.toString());
        assert.strictEqual(borrowingVaults.collateralVaultRay.toString(), borrowingGlobalAccounts.collateralVaultRay.toString());
        assert.strictEqual(borrowingVaults.srmMint.toString(), borrowingGlobalAccounts.srmMint.toString());
        assert.strictEqual(borrowingVaults.ethMint.toString(), borrowingGlobalAccounts.ethMint.toString());
        assert.strictEqual(borrowingVaults.btcMint.toString(), borrowingGlobalAccounts.btcMint.toString());
        assert.strictEqual(borrowingVaults.rayMint.toString(), borrowingGlobalAccounts.rayMint.toString());

        const globalConfig = await getGlobalConfig(program, borrowingGlobalAccounts.globalConfig.publicKey);
        assert.strictEqual(globalConfig.version, 0);
        assert.strictEqual(globalConfig.isBorrowingAllowed, true);
        assert.strictEqual(globalConfig.borrowLimitUsdh.toString(), "1000");
    });

    it('tests_borrowing_setup_user_accounts', async () => {

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(10, "SOL"));

        const borrowingGlobalAccounts = await set_up.setUpBorrowingGlobalAccounts(
            provider,
            initialMarketOwner,
            program);

        const userAccounts = await set_up.setUpBorrowingUserAccounts(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            borrowingGlobalAccounts);

        console.log(`Associated Stablecoin Mint ${userAccounts.stablecoinAta.toString()}`);
        console.log(`Trove Data Account ${userAccounts.userMetadata.toString()}`);

    });

    it('tests_borrowing_initialize_trove', async () => {

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


        const userAccounts = await set_up.setUpBorrowingUserAccounts(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            borrowingGlobalAccounts);

        await instructions_borrow
            .initializeTrove(
                program,
                user.publicKey,
                userAccounts.userMetadata,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                userAccounts.stablecoinAta,
                [user]);

        await displayTrove(program, userAccounts);

        let userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);
        assert.strictEqual(userMetadata.userId, 0);

        displayUserMetadata(userMetadata);

        assert.strictEqual(userMetadata.owner.toString(), user.publicKey.toString());
        assert.strictEqual(userMetadata.borrowingMarketState.toString(), borrowingGlobalAccounts.borrowingMarketState.publicKey.toString());
        assert.strictEqual(userMetadata.metadataPk.toString(), userAccounts.userMetadata.publicKey.toString());
        assert.strictEqual(userMetadata.stablecoinAta.toString(), userAccounts.stablecoinAta.toString());
    });

    it('tests_borrowing_initialize_two_troves', async () => {

        const userOne = anchor.web3.Keypair.generate();
        const userTwo = anchor.web3.Keypair.generate();

        await provider.connection.requestAirdrop(userOne.publicKey, utils.collToLamports(10, "SOL"));
        await provider.connection.requestAirdrop(userTwo.publicKey, utils.collToLamports(10, "SOL"));

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

        const userOneAccounts = await set_up.setUpBorrowingUserAccounts(
            provider,
            userOne.publicKey,
            [userOne],
            userOne.publicKey,
            borrowingGlobalAccounts);

        const userTwoAccounts = await set_up.setUpBorrowingUserAccounts(
            provider,
            userTwo.publicKey,
            [userTwo],
            userTwo.publicKey,
            borrowingGlobalAccounts);

        await instructions_borrow
            .initializeTrove(
                program,
                userOne.publicKey,
                userOneAccounts.userMetadata,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                userOneAccounts.stablecoinAta,
                [userOne]);

        await instructions_borrow
            .initializeTrove(
                program,
                userTwo.publicKey,
                userTwoAccounts.userMetadata,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                userTwoAccounts.stablecoinAta,
                [userTwo]);


        let userOneMetadata = await getUserMetadata(program, userOneAccounts.userMetadata.publicKey);
        let userTwoMetadata = await getUserMetadata(program, userTwoAccounts.userMetadata.publicKey);
        assert.strictEqual(userOneMetadata.userId, 0);
        assert.strictEqual(userTwoMetadata.userId, 1);

        const borrowingMarketState = await getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        assert.strictEqual(borrowingMarketState.numUsers, 2);

        console.log(`UserMetadata ${JSON.stringify(userOneMetadata)}`);
        console.log(`UserMetadata ${JSON.stringify(userTwoMetadata)}`);

        assert.strictEqual(userOneMetadata.owner.toString(), userOne.publicKey.toString());
        assert.strictEqual(userTwoMetadata.owner.toString(), userTwo.publicKey.toString());
    });

    it('tests_borrowing_deposit_collateral_expanded', async () => {

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

        console.log(`Initialized trove`);
        console.log(`Before Deposit Collateral`);

        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        // deposit 5 SOL
        let depositSol = 5;
        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSol,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositSol, "SOL"),
                [user]);

        console.log(`After Deposit Collateral`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        let userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

        console.log(`After Deposit Collateral`, userMetadata.depositedCollateral.sol);
        console.log(`After Deposit Collateral`, userMetadata.borrowedStablecoin);

        assert.strictEqual(userMetadata.inactiveCollateral.sol, utils.collToLamports(depositSol, "SOL"));
        assert.strictEqual(userMetadata.borrowedStablecoin, 0);

    });

    it('tests_borrowing_deposit_collateral_simple', async () => {
        let {
            user,
            userAccounts,
            borrowingGlobalAccounts
        } = await operations_borrowing.setUpMarketWithEmptyUser(provider, program, initialMarketOwner);

        // deposit 5 SOL
        let depositSol = 5;
        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSol,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositSol, "SOL"),
                [user]);

        console.log(`After Deposit Collateral`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        let userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

        console.log(`After Deposit Collateral`, userMetadata.depositedCollateral.sol);
        console.log(`After Deposit Collateral`, userMetadata.borrowedStablecoin);

        assert.strictEqual(userMetadata.inactiveCollateral.sol, utils.collToLamports(depositSol, "SOL"));
        assert.strictEqual(userMetadata.borrowedStablecoin, 0);
    });

    it('tests_borrowing_deposit_collateral_eth', async () => {
        let {
            user,
            userAccounts,
            borrowingGlobalAccounts
        } = await operations_borrowing.setUpMarketWithEmptyUser(provider, program, initialMarketOwner);

        // deposit 5 ETH
        let depositEth = 5;
        await operations_borrowing.mintToAta(provider, borrowingGlobalAccounts, userAccounts, "ETH", utils.collToLamports(depositEth + 1, "ETH"));

        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultEth,
                userAccounts.ethAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositEth, "ETH"),
                [user],
                "ETH");

        console.log(`After Deposit Collateral`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        let userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

        console.log(`Deposited collateral sol `, userMetadata.inactiveCollateral.sol);
        console.log(`Deposited collateral eth `, userMetadata.inactiveCollateral.eth);
        console.log(`Borrowed stablecoin `, userMetadata.borrowedStablecoin);

        assert.strictEqual(userMetadata.inactiveCollateral.eth, utils.collToLamports(depositEth, "ETH"));
        assert.strictEqual(userMetadata.borrowedStablecoin, 0);
    });

    it('tests_borrowing_deposit_collateral_sol_eth_btc', async () => {
        let {
            user,
            userAccounts,
            borrowingGlobalAccounts
        } = await operations_borrowing.setUpMarketWithEmptyUser(provider, program, initialMarketOwner);

        let depositEth = 7;
        let depositBtc = 6;
        await operations_borrowing.mintToAta(provider, borrowingGlobalAccounts, userAccounts, "ETH", utils.collToLamports(depositEth, "ETH"))
        await operations_borrowing.mintToAta(provider, borrowingGlobalAccounts, userAccounts, "BTC", utils.collToLamports(depositBtc, "BTC"))

        let ethBalance = await provider.connection.getTokenAccountBalance(userAccounts.ethAta);
        let btcBalance = await provider.connection.getTokenAccountBalance(userAccounts.btcAta);
        // @ts-ignore
        assert.strictEqual(depositEth, Number.parseFloat(ethBalance.value.uiAmountString))
        // @ts-ignore
        assert.strictEqual(depositBtc, Number.parseFloat(btcBalance.value.uiAmountString))

        // deposit 5 SOL
        let depositSol = 5;
        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSol,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositSol, "SOL"),
                [user],
                "SOL");

        // deposit 7 ETH
        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultEth,
                userAccounts.ethAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositEth, "ETH"),
                [user],
                "ETH");

        // deposit 6 BTC
        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultBtc,
                userAccounts.btcAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositBtc, "BTC"),
                [user],
                "BTC");

        console.log(`After Deposit Collateral`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        let userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

        console.log(`Deposited collateral sol `, userMetadata.inactiveCollateral.sol);
        console.log(`Deposited collateral btc `, userMetadata.inactiveCollateral.btc);
        console.log(`Deposited collateral eth `, userMetadata.inactiveCollateral.eth);
        console.log(`Borrowed stablecoin `, userMetadata.borrowedStablecoin);

        assert.strictEqual(userMetadata.inactiveCollateral.eth, utils.collToLamports(depositEth, "ETH"));
        assert.strictEqual(userMetadata.inactiveCollateral.btc, utils.collToLamports(depositBtc, "BTC"));
        assert.strictEqual(userMetadata.inactiveCollateral.sol, utils.collToLamports(depositSol, "SOL"));
        assert.strictEqual(userMetadata.borrowedStablecoin, 0);

        {
            // user
            let ethBalance = await provider.connection.getTokenAccountBalance(userAccounts.ethAta);
            let btcBalance = await provider.connection.getTokenAccountBalance(userAccounts.btcAta);
            // @ts-ignore
            assert.strictEqual(0, Number.parseFloat(ethBalance.value.uiAmountString))
            // @ts-ignore
            assert.strictEqual(0, Number.parseFloat(btcBalance.value.uiAmountString))
        }

        {
            // vaults
            let ethBalance = await provider.connection.getTokenAccountBalance(borrowingGlobalAccounts.collateralVaultEth);
            let btcBalance = await provider.connection.getTokenAccountBalance(borrowingGlobalAccounts.collateralVaultBtc);
            // @ts-ignore
            assert.strictEqual(depositEth, Number.parseFloat(ethBalance.value.uiAmountString))
            // @ts-ignore
            assert.strictEqual(depositBtc, Number.parseFloat(btcBalance.value.uiAmountString))
        }
    });

    it('tests_borrowing_deposit_collateral_then_withdraw_eth', async () => {
        let {
            user,
            userAccounts,
            borrowingGlobalAccounts
        } = await operations_borrowing.setUpMarketWithEmptyUser(provider, program, initialMarketOwner);
        await displayBorrowingVaults(program, borrowingGlobalAccounts.borrowingVaults.publicKey);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        let mintEth = 7;
        let depositEth = 5;
        let withdrawEth = 3;

        await operations_borrowing.mintToAta(provider, borrowingGlobalAccounts, userAccounts, "ETH", utils.collToLamports(mintEth, "ETH"));

        // deposit 5 ETH
        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultEth,
                userAccounts.ethAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositEth, "ETH"),
                [user],
                "ETH");

        console.log(`Deposited ETH`);
        await instructions_borrow
            .withdrawCollateral(
                program,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultEth,
                userAccounts.ethAta,
                pythPrices,
                utils.collToLamports(withdrawEth, "ETH"),
                [user],
                "ETH");

        console.log(`Withdrew ETH`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        await displayBorrowingVaults(program, borrowingGlobalAccounts.borrowingVaults.publicKey);

        let userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

        console.log(`After Dep/With Collateral eth`, userMetadata.inactiveCollateral.eth);
        console.log(`After Dep/With Collateral eth`, userMetadata.borrowedStablecoin);

        assert.strictEqual(userMetadata.inactiveCollateral.eth, utils.collToLamports(depositEth - withdrawEth, "ETH"));
        assert.strictEqual(userMetadata.borrowedStablecoin, 0);

        const userEthBalance = await getTokenAccountBalance(program, userAccounts.ethAta);
        assert.strictEqual(userEthBalance, mintEth - depositEth + withdrawEth);
        const vaultEthBalance = await getTokenAccountBalance(program, borrowingGlobalAccounts.collateralVaultEth);
        assert.strictEqual(vaultEthBalance, depositEth - withdrawEth);
        const { inactiveCollateral } = await getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        assert.strictEqual(inactiveCollateral.eth, utils.collToLamports(depositEth - withdrawEth, "ETH"));
    });

    it('tests_borrowing_deposit_collateral_then_withdraw_ray_ftt_srm', async () => {
        let {
            user,
            userAccounts,
            borrowingGlobalAccounts
        } = await operations_borrowing.setUpMarketWithEmptyUser(provider, program, initialMarketOwner);
        await displayBorrowingVaults(program, borrowingGlobalAccounts.borrowingVaults.publicKey);
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        let mintSrm = 7;
        let depositSrm = 5;
        let withdrawSrm = 3;

        let mintRay = 9;
        let depositRay = 4;
        let withdrawRay = 3;

        let mintFtt = 12;
        let depositFtt = 10;
        let withdrawFtt = 1;

        await operations_borrowing.mintToAta(provider, borrowingGlobalAccounts, userAccounts, "SRM", utils.collToLamports(mintSrm, "SRM"))
        await operations_borrowing.mintToAta(provider, borrowingGlobalAccounts, userAccounts, "RAY", utils.collToLamports(mintRay, "SRM"))
        await operations_borrowing.mintToAta(provider, borrowingGlobalAccounts, userAccounts, "FTT", utils.collToLamports(mintFtt, "SRM"))

        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSrm,
                userAccounts.srmAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositSrm, "SRM"),
                [user],
                "SRM");

        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultFtt,
                userAccounts.fttAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositFtt, "FTT"),
                [user],
                "FTT");

        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultRay,
                userAccounts.rayAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositRay, "RAY"),
                [user],
                "RAY");

        console.log(`Deposited RAY SRM FTT`);

        await instructions_borrow
            .withdrawCollateral(
                program,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultFtt,
                userAccounts.fttAta,
                pythPrices,
                utils.collToLamports(withdrawFtt, "FTT"),
                [user],
                "FTT");
        await instructions_borrow
            .withdrawCollateral(
                program,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSrm,
                userAccounts.srmAta,
                pythPrices,
                utils.collToLamports(withdrawSrm, "SRM"),
                [user],
                "SRM");
        await instructions_borrow
            .withdrawCollateral(
                program,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultRay,
                userAccounts.rayAta,
                pythPrices,
                utils.collToLamports(withdrawRay, "RAY"),
                [user],
                "RAY");

        console.log(`Withdrew RAY SRM FTT`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
        await displayBorrowingVaults(program, borrowingGlobalAccounts.borrowingVaults.publicKey);

        let userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

        console.log(`After Dep/With Collateral eth`, userMetadata.inactiveCollateral.eth);
        console.log(`After Dep/With Collateral srm`, userMetadata.inactiveCollateral.srm);
        console.log(`After Dep/With Collateral ftt`, userMetadata.inactiveCollateral.ftt);
        console.log(`After Dep/With Collateral ray`, userMetadata.inactiveCollateral.ray);
        console.log(`After Dep/With Collateral eth`, userMetadata.borrowedStablecoin);

        assert.strictEqual(userMetadata.inactiveCollateral.srm, utils.collToLamports(depositSrm - withdrawSrm, "SRM"));
        assert.strictEqual(userMetadata.inactiveCollateral.ftt, utils.collToLamports(depositFtt - withdrawFtt, "FTT"));
        assert.strictEqual(userMetadata.inactiveCollateral.ray, utils.collToLamports(depositRay - withdrawRay, "RAY"));
        assert.strictEqual(userMetadata.borrowedStablecoin, 0);

        {
            // user
            let rayBalance = await getTokenAccountBalance(program, userAccounts.rayAta);
            let srmBalance = await getTokenAccountBalance(program, userAccounts.srmAta);
            let fttBalance = await getTokenAccountBalance(program, userAccounts.fttAta);
            assert.strictEqual(mintRay - depositRay + withdrawRay, rayBalance);
            assert.strictEqual(mintSrm - depositSrm + withdrawSrm, srmBalance);
            assert.strictEqual(mintFtt - depositFtt + withdrawFtt, fttBalance);
        }

        {
            // vaults
            let rayBalance = await getCollateralVaultBalance(program, borrowingGlobalAccounts.borrowingVaults.publicKey, "RAY");
            let srmBalance = await getCollateralVaultBalance(program, borrowingGlobalAccounts.borrowingVaults.publicKey, "SRM");
            let fttBalance = await getCollateralVaultBalance(program, borrowingGlobalAccounts.borrowingVaults.publicKey, "FTT");
            assert.strictEqual(depositRay - withdrawRay, rayBalance);
            assert.strictEqual(depositFtt - withdrawFtt, fttBalance);
            assert.strictEqual(depositSrm - withdrawSrm, srmBalance);
        }

        {
            // state
            let borrowingMarketState = await getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
            assert.strictEqual(borrowingMarketState.inactiveCollateral.ray, utils.collToLamports(depositRay - withdrawRay, "RAY"));
            assert.strictEqual(borrowingMarketState.inactiveCollateral.ftt, utils.collToLamports(depositFtt - withdrawFtt, "FTT"));
            assert.strictEqual(borrowingMarketState.inactiveCollateral.srm, utils.collToLamports(depositSrm - withdrawSrm, "SRM"));
        }
    });

    it('tests_borrowing_when_borrow_less_than_minimum_then_fail', async () => {

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(10, "SOL"));
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);
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

        console.log(`Initized trove`);

        console.log(`Before Deposit Collateral`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        // deposit 5 SOL
        let depositSol = 5;
        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSol,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositSol, "SOL"),
                [user]);

        console.log(`After Deposit Collateral`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        let borrowStablecoin = 30;

        try {
            await instructions_borrow
                .borrowStablecoin(
                    program,
                    user.publicKey,
                    userAccounts.userMetadata.publicKey,
                    borrowingGlobalAccounts.stablecoinMint,
                    userAccounts.stablecoinAta,
                    borrowingGlobalAccounts.borrowingMarketState.publicKey,
                    borrowingGlobalAccounts.borrowingVaults.publicKey,
                    borrowingGlobalAccounts.stakingPoolState.publicKey,
                    borrowingGlobalAccounts.borrowingFeesVault,
                    stakingAccounts.treasuryVault,
                    pythPrices,
                    utils.decimalToU64(borrowStablecoin),
                    [user]);
            assert.fail("Should not reach this");
        } catch (error) {
            console.log("Done")
        }

    });

    it('tests_borrowing_borrow_stablecoin', async () => {

        const user = anchor.web3.Keypair.generate();
        await provider.connection.requestAirdrop(user.publicKey, utils.collToLamports(10, "SOL"));
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);
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

        console.log(`Initized trove`);

        console.log(`Before Deposit Collateral`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        // deposit 5 SOL
        let depositSol = 5;
        await instructions_borrow
            .depositCollateral(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.collateralVaultSol,
                user.publicKey,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                utils.collToLamports(depositSol, "SOL"),
                [user]);

        console.log(`After Deposit Collateral`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        let borrowStablecoin = 300;
        await instructions_borrow
            .borrowStablecoin(
                program,
                user.publicKey,
                userAccounts.userMetadata.publicKey,
                borrowingGlobalAccounts.stablecoinMint,
                userAccounts.stablecoinAta,
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                borrowingGlobalAccounts.borrowingVaults.publicKey,
                borrowingGlobalAccounts.stakingPoolState.publicKey,
                borrowingGlobalAccounts.borrowingFeesVault,
                stakingAccounts.treasuryVault,
                pythPrices,
                utils.decimalToU64(borrowStablecoin),
                [user]);

        console.log(`After Borrow Stable`);
        await displayTrove(program, userAccounts);
        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        let userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

        console.log(`After Borrow stable`, userMetadata.depositedCollateral.sol);
        console.log(`After Borrow stable`, userMetadata.borrowedStablecoin);

        assert.strictEqual(userMetadata.depositedCollateral.sol, utils.collToLamports(depositSol, "SOL"));
        assert.strictEqual(userMetadata.borrowedStablecoin, utils.decimalToU64(301.5));

    });

    it('tests_borrowing_when_borrow_not_initialized_trove_then_error', async () => {

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

        console.log('Initialized market');

        await displayBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);

        console.log(`Created Global Accounts`);


        const userAccounts = await set_up.setUpBorrowingUserAccounts(
            provider,
            user.publicKey,
            [user],
            user.publicKey,
            borrowingGlobalAccounts);

        const stakingAccounts = await operations_staking.initialiseStakingPool(
            provider,
            program,
            initialMarketOwner,
            borrowingGlobalAccounts,
            1500);

        console.log(`Created User Accounts`);

        // Should not be able to deposit if not initialized trove
        try {
            await instructions_borrow
                .depositCollateral(
                    program,
                    user.publicKey,
                    userAccounts.userMetadata.publicKey,
                    borrowingGlobalAccounts.collateralVaultSol,
                    user.publicKey,
                    borrowingGlobalAccounts.borrowingMarketState.publicKey,
                    borrowingGlobalAccounts.borrowingVaults.publicKey,
                    utils.collToLamports(5, "SOL"),
                    [user]);
            assert.fail("Should not reach this");
        } catch (error) {
            console.log("Done")
        }

    });

    it('tests_borrowing_when_repay_loan_rest_debt_is_less than_minimum_then_fail', async () => {

        const borrowingMarkets = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowingMarkets.borrowingAccounts;
        const stakingPoolAccounts = borrowingMarkets.stakingPoolAccounts;
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const { borrower, borrowerAccounts, borrowerInitialBalance } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 10],
        ]));

        // deposit SOL
        const depositSol = 5;
        await operations_borrowing.depositCollateral(provider, program, depositSol, borrower, borrowerAccounts, borrowingGlobalAccounts);
        await sleep(1000)
        await assertGlobalCollateral(
            program,
            provider,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),             // active collateral
            new Map<CollateralToken, number>([["SOL", 5]]),   // inactive collateral
        );

        // borrow stable
        const borrowStablecoin = 300;
        await operations_borrowing.borrow(provider, program, borrowStablecoin, borrower, borrowerAccounts, borrowingGlobalAccounts, stakingPoolAccounts, pythPrices);

        // repay stable
        const repayStablecoin = 200;

        try {
            await operations_borrowing.repay(provider, program, repayStablecoin, borrower, borrowerAccounts, borrowingGlobalAccounts, pythPrices);
            assert.fail("Should not reach this");
        } catch (error) {
            console.log("Done")
        }

    });

    it('tests_borrowing_repay_loan', async () => {

        const borrowMarket = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowMarket.borrowingAccounts;
        const stakingAccounts = borrowMarket.stakingPoolAccounts;
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const { borrower, borrowerAccounts, borrowerInitialBalance } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 10],
        ]));

        // deposit SOL
        const depositSol = 5;
        await operations_borrowing.depositCollateral(provider, program, depositSol, borrower, borrowerAccounts, borrowingGlobalAccounts);
        await sleep(1000)
        await assertGlobalCollateral(
            program,
            provider,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),             // active collateral
            new Map<CollateralToken, number>([["SOL", 5]]),   // inactive collateral
        );

        // borrow stable
        const borrowStablecoin = 300;
        await operations_borrowing.borrow(provider, program, borrowStablecoin, borrower, borrowerAccounts, borrowingGlobalAccounts, stakingAccounts, pythPrices);

        // repay stable
        const repayStablecoin = 100;
        await operations_borrowing.repay(provider, program, repayStablecoin, borrower, borrowerAccounts, borrowingGlobalAccounts, pythPrices);

        let solAccount = utils.lamportsToColl((await provider.connection.getAccountInfo(borrower.publicKey))?.lamports, "SOL");
        console.log("Sol balance after repay", solAccount);

        // assert balances
        await assertBorrowerBalance(provider, program, borrower, borrowerAccounts, borrowingGlobalAccounts, 201.5, borrowerInitialBalance - 5, 200);
        await assertBorrowerCollateral(provider, program, borrower, borrowerAccounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", depositSol]
        ]))
    });

    it('tests_borrowing_users_deposit_borrow_and_repay', async () => {

        const borrowMarket = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowMarket.borrowingAccounts;
        const stakingAccounts = borrowMarket.stakingPoolAccounts;
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const { borrower: user1, borrowerAccounts: user1Accounts, borrowerInitialBalance: user1InitialBalance } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 12],
        ]));
        const { borrower: user2, borrowerAccounts: user2Accounts, borrowerInitialBalance: user2InitialBalance } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 22],
        ]));

        // deposit SOL
        const depositUser1Sol = 10;
        await operations_borrowing.depositCollateral(provider, program, depositUser1Sol, user1, user1Accounts, borrowingGlobalAccounts);
        const depositUser2Sol = 20;
        await operations_borrowing.depositCollateral(provider, program, depositUser2Sol, user2, user2Accounts, borrowingGlobalAccounts);
        await sleep(1000)
        await assertGlobalCollateral(
            program, provider,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),
            new Map<CollateralToken, number>([["SOL", 30]]),
        );

        // borrow stable
        const borrowUser1Stablecoin = 500;
        await operations_borrowing.borrow(provider, program, borrowUser1Stablecoin, user1, user1Accounts, borrowingGlobalAccounts, stakingAccounts, pythPrices);
        const borrowUser2Stablecoin = 300;
        await operations_borrowing.borrow(provider, program, borrowUser2Stablecoin, user2, user2Accounts, borrowingGlobalAccounts, stakingAccounts, pythPrices);
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, 804);

        // repay stable
        const repayUser1Stablecoin = 250;
        await operations_borrowing.repay(provider, program, repayUser1Stablecoin, user1, user1Accounts, borrowingGlobalAccounts, pythPrices);
        const repayUser2Stablecoin = 50;
        await operations_borrowing.repay(provider, program, repayUser2Stablecoin, user2, user2Accounts, borrowingGlobalAccounts, pythPrices);
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, 504);

        // assert balances
        await assertBorrowerBalance(provider, program, user1, user1Accounts, borrowingGlobalAccounts, 252.5, user1InitialBalance - 10, 250);
        await assertBorrowerCollateral(provider, program, user1, user1Accounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", depositUser1Sol]
        ]))
        await assertBorrowerBalance(provider, program, user2, user2Accounts, borrowingGlobalAccounts, 251.5, user2InitialBalance - 20, 250);
        await assertBorrowerCollateral(provider, program, user2, user2Accounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", depositUser2Sol]
        ]))
    });

    it('tests_borrowing_users_deposit_borrow_and_withdraw_collateral', async () => {

        const borrowMarket = await operations_borrowing.initialiseBorrowingMarkets(env);
        const borrowingGlobalAccounts = borrowMarket.borrowingAccounts;
        const stakingAccounts = borrowMarket.stakingPoolAccounts;
        const pythPrices = await set_up.setUpPythPrices(provider, pyth);

        const { borrower: user1, borrowerAccounts: user1Accounts, borrowerInitialBalance: user1InitialBalance } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 12],
        ]));
        const { borrower: user2, borrowerAccounts: user2Accounts, borrowerInitialBalance: user2InitialBalance } = await newBorrowingUser(env, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 22],
        ]));

        // deposit SOL
        const depositUser1Sol = 10;
        await operations_borrowing.depositCollateral(provider, program, depositUser1Sol, user1, user1Accounts, borrowingGlobalAccounts);
        const depositUser2Sol = 20;
        await operations_borrowing.depositCollateral(provider, program, depositUser2Sol, user2, user2Accounts, borrowingGlobalAccounts);
        await sleep(1000)
        await assertGlobalCollateral(
            program, provider,
            borrowingGlobalAccounts.borrowingMarketState.publicKey,
            borrowingGlobalAccounts.borrowingVaults.publicKey,
            new Map<CollateralToken, number>([]),
            new Map<CollateralToken, number>([["SOL", 30]]),
        );

        // borrow stable
        const borrowUser1Stablecoin = 500;
        await operations_borrowing.borrow(provider, program, borrowUser1Stablecoin, user1, user1Accounts, borrowingGlobalAccounts, stakingAccounts, pythPrices);
        const borrowUser2Stablecoin = 200;
        await operations_borrowing.borrow(provider, program, borrowUser2Stablecoin, user2, user2Accounts, borrowingGlobalAccounts, stakingAccounts, pythPrices);
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, 703.5);

        // withdraw collateral
        const withdrawUser1Sol = 1;
        await operations_borrowing.withdrawSolCollateral(provider, program, withdrawUser1Sol, user1, user1Accounts, borrowingGlobalAccounts, pythPrices);
        const withdrawUser2Sol = 15;
        await operations_borrowing.withdrawSolCollateral(provider, program, withdrawUser2Sol, user2, user2Accounts, borrowingGlobalAccounts, pythPrices);
        await sleep(1999);
        await assertGlobalCollateral(program, provider, borrowingGlobalAccounts.borrowingMarketState.publicKey, borrowingGlobalAccounts.borrowingVaults.publicKey, new Map<CollateralToken, number>([
            ["SOL", depositUser1Sol + depositUser2Sol - withdrawUser1Sol - withdrawUser2Sol]
        ]));

        await sleep(1999);
        await assertGlobalDebt(program, borrowingGlobalAccounts.borrowingMarketState.publicKey, 703.5);

        // assert balances
        await assertBorrowerBalance(provider, program, user1, user1Accounts, borrowingGlobalAccounts, 502.5, user1InitialBalance - 9, 500);
        await assertBorrowerCollateral(provider, program, user1, user1Accounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 9]
        ]))
        await assertBorrowerBalance(provider, program, user2, user2Accounts, borrowingGlobalAccounts, 201, user2InitialBalance - 5, 200);
        await assertBorrowerCollateral(provider, program, user2, user2Accounts, borrowingGlobalAccounts, new Map<CollateralToken, number>([
            ["SOL", 5]
        ]))
    });
});

