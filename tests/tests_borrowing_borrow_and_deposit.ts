import * as assert from "assert";
import { newBorrowingUser, initialiseBorrowingMarkets, newLoanee, airdropToUser } from "./operations_borrowing";
import { setUpProgram, Env, setUpPrices, updatePythPrices } from "../src/set_up";
import { getBorrowingMarket, getUserState, collateralVaultBalance, feesVaultBalance, stablecoinBalance, treasuryVaultBalance } from "./data_provider";
import { CollateralToken } from "./types";
import { depositAndBorrow, setUpMarketWithUser, map } from "./operations_borrowing";
import { initalizeMarketAndStakingPool } from "./operations_staking";
import { collToLamports, decimalToU64 } from "../src/utils";
import { sleep } from "@project-serum/common";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { setupMaster } from "cluster";

chai.use(chaiAsPromised)

describe('tests_deposit_and_borrow', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as Env;

    it('tests_deposit_and_borrow_open_single_position_normal_mode', async () => {
        // This test checks that an atomic transaction of depositAndBorrow
        // creates a new user correctly, state is updated and collateral
        // is deposited and stablecoin is minted

        const { borrowingMarketAccounts, stakingPoolAccounts } = await initalizeMarketAndStakingPool(env);
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.7 });

        const borrowerAccounts = await newBorrowingUser(env, borrowingMarketAccounts, map.from(3000.0, "SOL"));
        const [borrow, deposit, asset] = [1000.0, 1000.0, "SOL" as CollateralToken];

        await depositAndBorrow(
            env,
            borrowingMarketAccounts,
            stakingPoolAccounts,
            borrowerAccounts,
            prices,
            borrow,
            deposit,
            asset);

        let market = await getBorrowingMarket(env.program, borrowingMarketAccounts);
        let user = await getUserState(env.program, borrowerAccounts);

        // Assert state
        let fee = 5.0;
        let treasury_fee = 5.0 * 0.15;
        assert.strictEqual(market.depositedCollateral.sol, sol.from(deposit));
        assert.strictEqual(market.stablecoinBorrowed, usdh.from(borrow + fee));
        assert.strictEqual(user.depositedCollateral.sol, sol.from(deposit));
        assert.strictEqual(user.borrowedStablecoin, usdh.from(borrow + fee));

        // Assert collateral
        await sleep(100);
        let collVaultSol = await collateralVaultBalance(env.program, borrowingMarketAccounts, "SOL");
        let feesVault = await feesVaultBalance(env.program, borrowingMarketAccounts);
        let treasuryVault = await treasuryVaultBalance(env.program, stakingPoolAccounts);
        let usdhBalance = await stablecoinBalance(env.program, borrowerAccounts);
        assert.strictEqual(usdhBalance, borrow);
        assert.strictEqual(collVaultSol, deposit);
        assert.strictEqual(feesVault, fee - treasury_fee);
        assert.strictEqual(treasuryVault, treasury_fee);

    });

    it('tests_deposit_and_borrow_open_first_position_into_recovery_mode_disallowed', async () => {
        // This test checks that an atomic transaction of depositAndBorrow
        // creates a new user correctly, state is updated and collateral
        // is deposited and stablecoin is minted

        const { borrowingMarketAccounts, stakingPoolAccounts } = await initalizeMarketAndStakingPool(env);
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.4 });

        const borrowerAccounts = await newBorrowingUser(env, borrowingMarketAccounts, map.from(2000.0, "SOL"));
        const [borrow, deposit, asset] = [1000.0, 1000.0, "SOL" as CollateralToken];

        await expect(depositAndBorrow(
            env,
            borrowingMarketAccounts,
            stakingPoolAccounts,
            borrowerAccounts,
            prices,
            borrow,
            deposit,
            asset)).to.be.rejectedWith("Operation brings system to recovery mode");

    });

    it('tests_deposit_and_borrow_adjust_existing_into_recovery_mode_disallowed', async () => {
        // Test checks that starting with a good user 
        // and then adjusting it's coll+debt down into recovery mode
        // is disallowed

        const [borrow, deposit, asset] = [1000.0, 1000.0, "SOL" as CollateralToken];
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.6 });

        let {
            borrowingUserState,
            borrowingAccounts,
            stakingAccounts,
        } = await setUpMarketWithUser(env, prices, borrow, deposit, asset);

        const [newBorrow, newDeposit, newAsset] = [1400.0, 400.0, "SOL" as CollateralToken];
        await expect(depositAndBorrow(
            env,
            borrowingAccounts,
            stakingAccounts,
            borrowingUserState,
            prices,
            newBorrow,
            newDeposit,
            newAsset)).to.be.rejectedWith("Operation brings system to recovery mode");

    });

    it('tests_deposit_and_borrow_improve_personal_cr_even_if_still_in_recovery', async () => {
        // This test checks that you can upgrade your balance, even if you are starting
        // from a recovery mode collateral ratio, and add more debt + collatearal
        // at 0 borrowing rate too.

        let { borrowingMarketAccounts, stakingPoolAccounts } = await initalizeMarketAndStakingPool(env);
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.6 });

        // two users with: 1000.0 USDH, 1000.0 SOL, px: 1.6 => 160% coll ratio
        const [borrow, deposit, asset] = [10000.0, 10000.0, "SOL" as CollateralToken];
        const firstBorrowerState = await newLoanee(env, borrowingMarketAccounts, stakingPoolAccounts, prices, borrow, map.from(deposit, asset));
        const secondBorrowerState = await newLoanee(env, borrowingMarketAccounts, stakingPoolAccounts, prices, borrow, map.from(deposit, asset));

        let fee = 50;
        let treasuryFee = 0.15 * fee;
        {
            // Assert global state
            let market = await getBorrowingMarket(env.program, borrowingMarketAccounts);
            assert.strictEqual(market.depositedCollateral.sol, sol.from(deposit * 2));
            assert.strictEqual(market.stablecoinBorrowed, usdh.from((borrow + fee) * 2));

            // Assert global collateral
            await sleep(100);
            let collVaultSol = await collateralVaultBalance(env.program, borrowingMarketAccounts, "SOL");
            let feesVault = await feesVaultBalance(env.program, borrowingMarketAccounts);
            assert.strictEqual(collVaultSol, deposit * 2);
            assert.strictEqual(feesVault, (fee - treasuryFee) * 2);

            for (const user of [firstBorrowerState, secondBorrowerState]) {
                let userMetadata = await getUserState(env.program, user);
                let usdhBalance = await stablecoinBalance(env.program, user);
                assert.strictEqual(userMetadata.depositedCollateral.sol, sol.from(deposit));
                assert.strictEqual(userMetadata.borrowedStablecoin, usdh.from(borrow + fee));
                assert.strictEqual(usdhBalance, borrow);
            }
        }

        // Now deposit + borrow more at 0% fee
        await updatePythPrices(pyth, prices, { solPrice: 1.4 });
        const [newBorrow, newDeposit] = [1000.0, 2000.0];
        await airdropToUser(env, borrowingMarketAccounts, secondBorrowerState, map.from(newDeposit + 1, asset));
        await depositAndBorrow(
            env,
            borrowingMarketAccounts,
            stakingPoolAccounts,
            secondBorrowerState,
            prices,
            newBorrow,
            newDeposit,
            asset);

        {
            // Assert global state
            let newFee = 0.0;
            let market = await getBorrowingMarket(env.program, borrowingMarketAccounts);
            assert.strictEqual(market.depositedCollateral.sol, sol.from(deposit * 2 + newDeposit));
            assert.strictEqual(market.stablecoinBorrowed, usdh.from((borrow + fee) * 2 + newBorrow + newFee));

            // Assert global collateral
            await sleep(100);
            let collVaultSol = await collateralVaultBalance(env.program, borrowingMarketAccounts, "SOL");
            let feesVault = await feesVaultBalance(env.program, borrowingMarketAccounts);
            assert.strictEqual(collVaultSol, deposit * 2 + newDeposit);
            assert.strictEqual(feesVault, (fee - treasuryFee) * 2);

            // Check second user only
            let userMetadata = await getUserState(env.program, secondBorrowerState);
            let usdhBalance = await stablecoinBalance(env.program, secondBorrowerState);
            assert.strictEqual(userMetadata.depositedCollateral.sol, sol.from(deposit + newDeposit));
            assert.strictEqual(userMetadata.borrowedStablecoin, usdh.from(borrow + fee + newBorrow + newFee));
            assert.strictEqual(usdhBalance, borrow + newBorrow);
        }


    });

    it('tests_deposit_and_borrow_adjust_existing_out_of_recovery_mode_allowed', async () => {
        const [borrow, deposit, asset] = [1000.0, 1000.0, "SOL" as CollateralToken];
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.6 });

        let {
            borrowingUserState,
            borrowingAccounts,
            stakingAccounts
        } = await setUpMarketWithUser(env, prices, borrow, deposit, asset);

        // This makes everything recovery mode
        await updatePythPrices(pyth, prices, { solPrice: 1.4 });

        const [newBorrow, newDeposit] = [1000.0, 2000.0];
        await airdropToUser(env, borrowingAccounts, borrowingUserState, map.from(newDeposit + 1, asset));
        await depositAndBorrow(
            env,
            borrowingAccounts,
            stakingAccounts,
            borrowingUserState,
            prices,
            newBorrow,
            newDeposit,
            asset);

        // Assert global state
        await sleep(100);
        let market = await getBorrowingMarket(env.program, borrowingAccounts);
        let userMetadata = await getUserState(env.program, borrowingUserState);
        let usdhBalance = await stablecoinBalance(env.program, borrowingUserState);
        let collVaultSol = await collateralVaultBalance(env.program, borrowingAccounts, "SOL");
        let feesVault = await feesVaultBalance(env.program, borrowingAccounts);

        let oldFee = 5.0;
        let treasuryFee = oldFee * 0.15;
        let newFee = 0.0;

        // Assert market
        assert.strictEqual(collVaultSol, deposit + newDeposit);
        assert.strictEqual(feesVault, (oldFee - treasuryFee) + newFee);
        assert.strictEqual(market.depositedCollateral.sol, sol.from(deposit + newDeposit));
        assert.strictEqual(market.stablecoinBorrowed, usdh.from(borrow + oldFee + newBorrow + newFee));

        // Check user balance
        assert.strictEqual(userMetadata.depositedCollateral.sol, sol.from(deposit + newDeposit));
        assert.strictEqual(userMetadata.borrowedStablecoin, usdh.from(borrow + oldFee + newBorrow + newFee));
        assert.strictEqual(usdhBalance, borrow + newBorrow);

    });

    it('tests_deposit_and_borrow_adjust_existing_in_normal_mode', async () => {

        const [borrow, deposit, asset] = [1000.0, 1000.0, "SOL" as CollateralToken];
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.6 });

        let {
            borrowingUserState,
            borrowingAccounts,
            stakingAccounts
        } = await setUpMarketWithUser(env, prices, borrow, deposit, asset);

        await airdropToUser(env, borrowingAccounts, borrowingUserState, map.from(deposit + 1, asset));
        await depositAndBorrow(
            env,
            borrowingAccounts,
            stakingAccounts,
            borrowingUserState,
            prices,
            borrow,
            deposit,
            asset);

        // Assert global state
        await sleep(100);
        let market = await getBorrowingMarket(env.program, borrowingAccounts);
        let userMetadata = await getUserState(env.program, borrowingUserState);
        let usdhBalance = await stablecoinBalance(env.program, borrowingUserState);
        let collVaultSol = await collateralVaultBalance(env.program, borrowingAccounts, "SOL");
        let feesVault = await feesVaultBalance(env.program, borrowingAccounts);

        let fee = 5.0;
        let treasuryfee = fee * 0.15;

        // Assert market
        assert.strictEqual(collVaultSol, deposit * 2);
        assert.strictEqual(feesVault, (fee - treasuryfee) * 2);
        assert.strictEqual(market.depositedCollateral.sol, sol.from(deposit * 2));
        assert.strictEqual(market.stablecoinBorrowed, usdh.from((borrow + fee) * 2));

        // Check user balance
        assert.strictEqual(userMetadata.depositedCollateral.sol, sol.from(deposit * 2));
        assert.strictEqual(userMetadata.borrowedStablecoin, usdh.from((borrow + fee) * 2));
        assert.strictEqual(usdhBalance, borrow * 2);

    });

    it('tests_deposit_and_borrow_open_single_position_recovery_mode_disallowed_below_150', async () => {
        const [borrow, deposit, asset] = [1000.0, 1000.0, "SOL" as CollateralToken];
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.6 });

        let {
            borrowingUserState,
            borrowingAccounts,
            stakingAccounts
        } = await setUpMarketWithUser(env, prices, borrow, deposit, asset);

        // This makes everything recovery mode
        await updatePythPrices(pyth, prices, { solPrice: 1.4 });

        const borrowerAccounts = await newBorrowingUser(env, borrowingAccounts, map.from(2000.0, "SOL"));

        await expect(depositAndBorrow(
            env,
            borrowingAccounts,
            stakingAccounts,
            borrowerAccounts,
            prices,
            borrow,
            deposit,
            asset)).to.be.rejectedWith("Insufficient collateral to cover debt");
    });

    it('tests_deposit_and_borrow_open_single_position_recovery_mode_allowed_above_150', async () => {

        const [borrow, deposit, asset] = [1000.0, 1000.0, "SOL" as CollateralToken];
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.6 });

        let {
            borrowingUserState,
            borrowingAccounts,
            stakingAccounts
        } = await setUpMarketWithUser(env, prices, borrow, deposit, asset);

        // This makes everything recovery mode
        await updatePythPrices(pyth, prices, { solPrice: 1.4 });

        // Try to open a position above 150%, must be allowed
        const secondBorrowerUserState = await newBorrowingUser(env, borrowingAccounts, map.from(4000.0, "SOL"));
        const [newDeposit] = [2000.0];
        await depositAndBorrow(
            env,
            borrowingAccounts,
            stakingAccounts,
            secondBorrowerUserState,
            prices,
            borrow,
            newDeposit,
            asset);

        // Assert global state
        await sleep(100);
        let market = await getBorrowingMarket(env.program, borrowingAccounts);
        let firstUserMetadata = await getUserState(env.program, borrowingUserState);
        let secondUserMetadata = await getUserState(env.program, secondBorrowerUserState);
        let firstUsdhBalance = await stablecoinBalance(env.program, borrowingUserState);
        let secondUsdhBalance = await stablecoinBalance(env.program, secondBorrowerUserState);
        let collVaultSol = await collateralVaultBalance(env.program, borrowingAccounts, "SOL");
        let feesVault = await feesVaultBalance(env.program, borrowingAccounts);
        let treasuryVault = await treasuryVaultBalance(env.program, stakingAccounts);

        let fee = 5.0;
        let treasuryFee = fee * 0.15;

        // Assert market
        assert.strictEqual(collVaultSol, deposit + newDeposit);
        assert.strictEqual(feesVault, fee - treasuryFee);
        assert.strictEqual(treasuryVault, treasuryFee);
        assert.strictEqual(market.depositedCollateral.sol, sol.from(deposit + newDeposit));
        assert.strictEqual(market.stablecoinBorrowed, usdh.from(borrow * 2 + fee));

        // Check first user balance
        assert.strictEqual(firstUserMetadata.depositedCollateral.sol, sol.from(deposit));
        assert.strictEqual(firstUserMetadata.borrowedStablecoin, usdh.from(borrow + fee));
        assert.strictEqual(firstUsdhBalance, borrow);

        // Check second user balance
        assert.strictEqual(secondUserMetadata.depositedCollateral.sol, sol.from(newDeposit));
        assert.strictEqual(secondUserMetadata.borrowedStablecoin, usdh.from(borrow));
        assert.strictEqual(secondUsdhBalance, borrow);

    });

    it('tests_deposit_and_borrow_adjusts_correctly_inactive_position', async () => {
        const [borrow, deposit, asset] = [0, 1000.0, "SOL" as CollateralToken];
        const prices = await setUpPrices(provider, pyth, { solPrice: 1.6 });

        let {
            borrowingUserState,
            borrowingAccounts,
            stakingAccounts
        } = await setUpMarketWithUser(env, prices, borrow, deposit, asset);

        let userMetadata = await getUserState(env.program, borrowingUserState);
        let newBorrow = 1000;
        assert.strictEqual(userMetadata.inactiveCollateral.sol, sol.from(deposit));
        await airdropToUser(env, borrowingAccounts, borrowingUserState, map.from(deposit + 1, asset));

        await depositAndBorrow(
            env,
            borrowingAccounts,
            stakingAccounts,
            borrowingUserState,
            prices,
            newBorrow,
            deposit,
            asset);

        await sleep(100);
        let market = await getBorrowingMarket(env.program, borrowingAccounts);
        let user = await getUserState(env.program, borrowingUserState);

        // Assert state
        let fee = 5.0;
        let treasuryFee = fee * 0.15;
        assert.strictEqual(market.depositedCollateral.sol, sol.from(deposit * 2));
        assert.strictEqual(market.stablecoinBorrowed, usdh.from(newBorrow + fee));
        assert.strictEqual(user.depositedCollateral.sol, sol.from(deposit * 2));
        assert.strictEqual(user.borrowedStablecoin, usdh.from(newBorrow + fee));

        // Assert collateral
        await sleep(100);
        let collVaultSol = await collateralVaultBalance(env.program, borrowingAccounts, "SOL");
        let feesVault = await feesVaultBalance(env.program, borrowingAccounts);
        let treasuryVault = await treasuryVaultBalance(env.program, stakingAccounts);
        let usdhBalance = await stablecoinBalance(env.program, borrowingUserState);
        assert.strictEqual(usdhBalance, newBorrow);
        assert.strictEqual(collVaultSol, deposit * 2);
        assert.strictEqual(feesVault, fee - treasuryFee);
        assert.strictEqual(treasuryVault, treasuryFee);

    });

});



namespace usdh {
    export function from(n: number): number {
        return decimalToU64(n)
    }
}

namespace sol {
    export function from(n: number): number {
        return collToLamports(n, "SOL")
    }
}