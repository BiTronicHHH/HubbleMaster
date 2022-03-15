import { AccountInfo, Keypair, PublicKey } from "@solana/web3.js";
import * as utils from '../src/utils';
import * as anchor from '@project-serum/anchor';
import * as assert from "assert";
import {
    getUserMetadata,
    getCollateralVaultBalance,
    getStabilityProviderAccount,
    getStakingPoolState,
    getBorrowingMarketState,
    getBorrowingVaults,
    getRedemptionsQueueData,
    getTokenAccountBalance,
    getStabilityPoolState
} from "./data_provider";
import { BorrowingGlobalAccounts, BorrowingUserAccounts, StabilityPoolAccounts, StabilityProviderAccounts, StakingPoolAccounts } from "../src/set_up";
import { CollateralToken } from "./types";
import { BN } from "@project-serum/anchor";
import { expect } from "chai";
import { mapToObj, tokenMapPrint, FACTOR, u64ToDecimal, lamportsToColl, collateralMapPrint } from "../src/utils";
import { displayUserMetadata } from "../src/utils_display";

export async function assertGlobalCollateral(
    program: anchor.Program,
    provider: anchor.Provider,
    borrowingMarketStatePk: PublicKey,
    borrowingVaultsKey: PublicKey,
    expectedCollateral: Map<CollateralToken, number>,
    expectedInactiveCollateral: Map<CollateralToken, number> = new Map<CollateralToken, number>([])) {

    const borrowingMarketState = await getBorrowingMarketState(program, borrowingMarketStatePk);
    const depositedCollateral = borrowingMarketState.depositedCollateral;
    const inactiveCollateral = borrowingMarketState.inactiveCollateral;

    let numCollateralDeposits = 0;
    Object.entries(depositedCollateral).forEach((entry) => {
        if (entry[1] > 0) {
            console.log("Actual Active", entry[0], entry[1]);
            numCollateralDeposits++;
        }
    })

    console.log("Expected active", JSON.stringify(expectedCollateral));

    let numInactiveCollateralDeposits = 0;
    Object.entries(inactiveCollateral).forEach((entry) => {
        if (entry[1] > 0) {
            console.log("Actual Inactive", entry[0], entry[1]);
            numInactiveCollateralDeposits++;
        }
    })
    console.log("Expected inactive", JSON.stringify(expectedInactiveCollateral));

    assert.strictEqual(expectedCollateral.size, numCollateralDeposits);
    assert.strictEqual(expectedInactiveCollateral.size, numInactiveCollateralDeposits);
    for (const [token, value] of expectedCollateral.entries()) {
        let expectedActive = value;
        let expectedInactive = 0;
        if (expectedInactiveCollateral.get(token)) {
            // @ts-ignore
            expectedInactive = expectedInactiveCollateral.get(token);
        }
        // @ts-ignore
        const borrowingMarketStateDepositedCollateralAmount = borrowingMarketState.depositedCollateral[token.toLowerCase()];
        const collateralVaultBalance = await getCollateralVaultBalance(program, borrowingVaultsKey, token);
        assert.strictEqual(collateralVaultBalance, expectedActive + expectedInactive,
            `${token} borrowing vault balance expected: '${expectedActive + expectedInactive}'. Actually was: ${collateralVaultBalance}`);
        assert.strictEqual(utils.lamportsToColl(borrowingMarketStateDepositedCollateralAmount, token as CollateralToken), value);
    }
}

export async function assertGlobalDebt(program: anchor.Program, borrowingMarketState: PublicKey, stablecoinDebt: number) {
    const borrowingMarketStateAccount = await program.account.borrowingMarketState.fetch(
        borrowingMarketState);
    // @ts-ignore
    assert.strictEqual(utils.u64ToDecimal(borrowingMarketStateAccount.stablecoinBorrowed.toNumber()), stablecoinDebt)
}

export async function assertBurningVaultBalance(provider: anchor.Provider, burningVault: PublicKey, balance: number) {
    const burningVaultAccount: any = await provider.connection.getTokenAccountBalance(burningVault);
    assert.strictEqual(Number.parseFloat(burningVaultAccount.value.uiAmountString), balance)
}

export async function assertStabilityPool(
    program: anchor.Program,
    provider: anchor.Provider,
    stabilityPoolState: PublicKey,
    stabilityPoolAccounts: StabilityPoolAccounts,
    numberOfUsers: number,
    totalStablecoinDeposits: number,
    solLiquidationsPending: number) {
    const stabilityPoolStateData = await getStabilityPoolState(program, stabilityPoolState);

    // Deposited Stability pool
    console.log(`StabilityPool stability provided -> ${utils.u64ToDecimal(stabilityPoolStateData.stablecoinDeposited)}`);
    assert.strictEqual(utils.u64ToDecimal(stabilityPoolStateData.stablecoinDeposited), totalStablecoinDeposits);
    assert.strictEqual(stabilityPoolStateData.numUsers, numberOfUsers);

    const stablecoinPoolBalance = await getTokenAccountBalance(program, stabilityPoolAccounts.stablecoinStabilityPoolVault);

    assert.strictEqual(totalStablecoinDeposits, stablecoinPoolBalance);

    // Gained stability pool collateral
    let solCollateralLiquidationsRewardsVault = await provider.connection.getAccountInfo(stabilityPoolAccounts.liquidationRewardsVaultSol);
    const rentExemptionSol = await provider.connection.getMinimumBalanceForRentExemption(9, "confirmed");

    assert.strictEqual(solCollateralLiquidationsRewardsVault?.lamports, rentExemptionSol + utils.collToLamports(solLiquidationsPending, "SOL"),
        `\nIncorrect SOL account balance for sol liq rewards vault ${stabilityPoolAccounts.liquidationRewardsVaultSol.toString()} .\nActual   ~ ${solCollateralLiquidationsRewardsVault?.lamports}\nExpected ~ ${rentExemptionSol + utils.collToLamports(solLiquidationsPending, "SOL")}`
    );
}

export async function assertBorrowerBalance(
    provider: anchor.Provider,
    program: anchor.Program,
    user: Keypair,
    userAccounts: BorrowingUserAccounts,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    stablecoinDebt: number,
    solBalance: number,
    stablecoinBalance: number) {

    const userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

    assert.strictEqual(userMetadata !== undefined, true, `Could not find position for user -> ${user.publicKey}`);

    displayUserMetadata(userMetadata);

    const solAccount = await provider.connection.getAccountInfo(user.publicKey);
    const stablecoinAccountPubKey = await utils.findAssociatedTokenAddress(user.publicKey, borrowingGlobalAccounts.stablecoinMint);
    const stablecoinAccount = await provider.connection.getTokenAccountBalance(stablecoinAccountPubKey);

    console.log("userMetadata.borrowedStablecoin", userMetadata.borrowedStablecoin);
    console.log("stablecoinAccountPubKey", stablecoinAccountPubKey.toString());
    console.log("FACTOR", FACTOR);
    console.log("userMetadata.borrowedStablecoin / FACTOR", userMetadata.borrowedStablecoin / FACTOR);

    console.log(`State user -> ${user.publicKey}`)
    console.log(`   Balances ->`);
    console.log(`       SOL -> ${utils.lamportsToColl(solAccount?.lamports, "SOL")}`);
    console.log(`       Stablecoin -> ${stablecoinAccount.value.uiAmountString}`);
    console.log(`   Debt stablecoin -> ${userMetadata.borrowedStablecoin / FACTOR}`);

    assert.strictEqual(utils.u64ToDecimal(userMetadata.borrowedStablecoin), stablecoinDebt);
    assert.ok(Math.abs(utils.lamportsToColl(solAccount?.lamports, "SOL") - solBalance) < 0.005,
        `\nIncorrect SOL account balance.\nUser - ${user.publicKey}\nExpected ~${solBalance} SOL but was ${utils.lamportsToColl(solAccount?.lamports, "SOL")} SOL`);
    // @ts-ignore
    assert.strictEqual(Number.parseFloat(stablecoinAccount.value.uiAmountString), stablecoinBalance);
}

export async function assertBorrowerCollateral(
    provider: anchor.Provider,
    program: anchor.Program,
    user: Keypair,
    userAccounts: BorrowingUserAccounts,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    expectedCollateral: Map<CollateralToken, number>,
    collateralType: "deposited" | "inactive" = "deposited"
): Promise<void> {

    const userMetadata = await getUserMetadata(program, userAccounts.userMetadata.publicKey);

    assert.strictEqual(userMetadata !== undefined, true, `Could not find position for user -> ${user.publicKey}`)

    const collateral = collateralType == "inactive" ? userMetadata.inactiveCollateral : userMetadata.depositedCollateral;
    console.log("Borrower deposited collateral sol", userMetadata.depositedCollateral.sol);
    console.log("Borrower deposited collateral eth", userMetadata.depositedCollateral.eth);
    console.log("Borrower deposited collateral btc", userMetadata.depositedCollateral.btc);
    console.log("Borrower deposited collateral ray", userMetadata.depositedCollateral.ray);
    console.log("Borrower deposited collateral srm", userMetadata.depositedCollateral.srm);
    console.log("Borrower deposited collateral ftt", userMetadata.depositedCollateral.ftt);

    console.log("Borrower inactive collateral sol", userMetadata.inactiveCollateral.sol);
    console.log("Borrower inactive collateral eth", userMetadata.inactiveCollateral.eth);
    console.log("Borrower inactive collateral btc", userMetadata.inactiveCollateral.btc);
    console.log("Borrower inactive collateral ray", userMetadata.inactiveCollateral.ray);
    console.log("Borrower inactive collateral srm", userMetadata.inactiveCollateral.srm);
    console.log("Borrower inactive collateral ftt", userMetadata.inactiveCollateral.ftt);

    displayUserMetadata(userMetadata);

    let actualBorrowerNumberOfCollateralTokens = 0;
    Object.values(collateral).forEach((value) => {
        if (value > 0) {
            actualBorrowerNumberOfCollateralTokens++;
        }
    })

    let expectedBorrowerNumberOfCollateralTokens = 0;
    console.log("expectedCollateral", JSON.stringify(expectedCollateral));
    expectedCollateral.forEach((value: number, key: CollateralToken) => {
        console.log("value", value);
        if (value > 0) {
            expectedBorrowerNumberOfCollateralTokens++;
        }
    })

    assert.strictEqual(actualBorrowerNumberOfCollateralTokens, expectedBorrowerNumberOfCollateralTokens,
        `Expected borrower '${userAccounts.userMetadata.publicKey.toBase58()}' number of collateral tokens: ${expectedBorrowerNumberOfCollateralTokens}. Actual: '${actualBorrowerNumberOfCollateralTokens}'\n` +
        `Expected collateral: ${JSON.stringify(mapToObj(expectedCollateral), undefined, 2)}\n` +
        `Actual collateral: ${JSON.stringify(collateralMapPrint(collateral), undefined, 2)}`
    );

    console.log(`State user -> ${user.publicKey}`)
    console.log(`   Collateral ->`);

    for (const [token, expectedAmount] of expectedCollateral.entries()) {
        // @ts-ignore
        const actualAmount = utils.lamportsToColl(collateral[token.toLowerCase()], token as CollateralToken);
        console.log(`       ${token} -> ${actualAmount}`);
        assert.strictEqual(actualAmount, expectedAmount,
            `${token} borrower balance expected: '${expectedAmount}'. Actual: '${actualAmount}'.\n` +
            `Expected collateral: ${JSON.stringify(mapToObj(expectedCollateral), undefined, 2)}\n` +
            `Actual collateral: ${JSON.stringify(collateralMapPrint(collateral), undefined, 2)}`
        );
    }
}

export async function assertStabilityProviderBalance(
    provider: anchor.Provider,
    program: anchor.Program,
    stabilityProvider: PublicKey,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    stabilityProviderAccounts: StabilityProviderAccounts,
    expectedStablecoinProvided: number,
    expectedStablecoinBalance: number
) {
    const stabilityProviderState = await getStabilityProviderAccount(program, stabilityProviderAccounts.stabilityProviderState.publicKey);
    const solAccount = await provider.connection.getAccountInfo(stabilityProvider);
    const stablecoinAccountPubKey = await utils.findAssociatedTokenAddress(stabilityProvider, borrowingGlobalAccounts.stablecoinMint);

    const stablecoinBalance = await getTokenAccountBalance(program, stablecoinAccountPubKey);

    const ethAta = await provider.connection.getTokenAccountBalance(stabilityProviderAccounts.ethAta);
    const srmAta = await provider.connection.getTokenAccountBalance(stabilityProviderAccounts.srmAta);
    const btcAta = await provider.connection.getTokenAccountBalance(stabilityProviderAccounts.btcAta);
    const fttAta = await provider.connection.getTokenAccountBalance(stabilityProviderAccounts.fttAta);
    const rayAta = await provider.connection.getTokenAccountBalance(stabilityProviderAccounts.rayAta);

    console.log(`Stability Provider State user -> ${stabilityProvider.toString()}`)
    console.log(`   Balances ->`);
    console.log(`       SOL -> ${utils.lamportsToColl(solAccount?.lamports, "SOL")}`);
    console.log(`       Stablecoin -> ${stablecoinBalance}`);
    console.log(`       ETH -> ${ethAta.value.uiAmountString}`);
    console.log(`       SRM -> ${srmAta.value.uiAmountString}`);
    console.log(`       BTC -> ${btcAta.value.uiAmountString}`);
    console.log(`       FTT -> ${fttAta.value.uiAmountString}`);
    console.log(`       RAY -> ${rayAta.value.uiAmountString}`);
    console.log(`   Deposited stablecoin -> ${utils.decimalToU64(stabilityProviderState.depositedStablecoin)}`);

    assert.strictEqual(u64ToDecimal(stabilityProviderState.depositedStablecoin), expectedStablecoinProvided);
    assert.strictEqual(stablecoinBalance, expectedStablecoinBalance);
}

export async function assertStakingPoolBalance(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingAccounts: BorrowingGlobalAccounts,
    stakingPoolAccounts: StakingPoolAccounts,
    totalHbbStaked: number,
    totalDistributedRewards: number,
    rewardsNotYetClaimed: number,
    rewardPerToken: number
) {
    const stakingPoolState = await getStakingPoolState(program, borrowingAccounts.stakingPoolState.publicKey);

    const stakingVaultAccount = await provider.connection.getTokenAccountBalance(stakingPoolAccounts.stakingVault);

    console.log(`HBB Staked -> ${stakingPoolState.totalStake}`);
    console.log(`Total distributed rewards -> ${stakingPoolState.totalDistributedRewards}`);
    console.log(`Rewards not yet claimed -> ${stakingPoolState.rewardsNotYetClaimed}`);
    console.log(`Reward per token -> ${stakingPoolState.rewardPerToken}`);


    assert.strictEqual(stakingPoolState.totalStake, totalHbbStaked, 'Total HBB staked assertion');
    assert.strictEqual(stakingPoolState.totalDistributedRewards, utils.decimalToU64(totalDistributedRewards), 'Total distributed rewards assertion');
    assert.strictEqual(stakingPoolState.rewardsNotYetClaimed, utils.decimalToU64(rewardsNotYetClaimed), 'Rewards not yet claimed assertion');
    assert.strictEqual(stakingPoolState.rewardPerToken, utils.decimalToU64(rewardPerToken) * 1000000, "Reward per token assertion");
    // assert.strictEqual(stakingPoolState.rewardPerToken, utils.decimalToU64(rewardPerToken), "Reward per token assertion");
    // @ts-ignore
    assert.strictEqual(Number.parseFloat(stakingVaultAccount.value.uiAmountString), utils.u64ToDecimal(totalHbbStaked), 'Staking vault balance assertion');

}

export async function assertStakerBalance(
    provider: anchor.Provider,
    program: anchor.Program,
    staker: PublicKey,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    // stakingPoolAccounts: StakingPoolAccounts,
    hbbBalance: number,
    stablecoinBalance: number,
) {
    const stakingPoolState = await getStakingPoolState(program, borrowingGlobalAccounts.stakingPoolState.publicKey);
    const solAccount = await provider.connection.getAccountInfo(staker);
    const stablecoinAccountPubKey = await utils.findAssociatedTokenAddress(staker, borrowingGlobalAccounts.stablecoinMint);
    const stablecoinAccount = await provider.connection.getTokenAccountBalance(stablecoinAccountPubKey);

    const hbbUserAccountPubkey = await utils.findAssociatedTokenAddress(staker, borrowingGlobalAccounts.hbbMint);
    const hbbUserAccount = await provider.connection.getTokenAccountBalance(hbbUserAccountPubkey);

    console.log(`Stability Provider State user -> ${staker.toString()}`)
    console.log(`   Balances ->`);
    console.log(`       SOL -> ${utils.lamportsToColl(solAccount?.lamports, "SOL")}`);
    console.log(`       Stablecoin -> ${stablecoinAccount.value.uiAmountString}`);
    console.log(`       Hbb -> ${hbbUserAccount.value.uiAmountString}`);

    // @ts-ignore
    assert.ok(utils.u64ToDecimal(stablecoinBalance) - Number.parseFloat(stablecoinAccount.value.uiAmountString) < 0.0001, "User stablecoin balance assertion");
    // @ts-ignore
    assert.strictEqual(Number.parseFloat(hbbUserAccount.value.uiAmountString), utils.u64ToDecimal(hbbBalance), "User hbb balance assertion");

}

export async function assertRedemptionsQueueSize(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    expectedSize: number,
) {
    const redemptionOrders = await getRedemptionsQueueData(program, borrowingGlobalAccounts.redemptionsQueue);
    const activeOrders = redemptionOrders.filter(order => order.status !== 0);
    assert.strictEqual(activeOrders.length, expectedSize, `Expected redemptions queue to have size: ${expectedSize} but was: ${activeOrders.length}.`);
}

export async function assertRedemptionsQueueOrderFilled(
    provider: anchor.Provider,
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    expectedCandidatesToFillers: { loaneeMetadata: PublicKey, fillerMetadata: PublicKey }[],
) {
    const redemptionOrders = await getRedemptionsQueueData(program, borrowingGlobalAccounts.redemptionsQueue);
    const fillingOrders = redemptionOrders.filter(order => order.status === 2);
    assert.strictEqual(fillingOrders.length, 1, `Expected 1 filling order on queue but there were: ${fillingOrders.length}.`);

    const expected: { loaneeMetadata: string, fillerMetadata: string }[] = expectedCandidatesToFillers.map((val) => {
        return {
            loaneeMetadata: val.loaneeMetadata.toBase58(),
            fillerMetadata: val.fillerMetadata.toBase58()
        };
    })

    const actual: { loaneeMetadata: string, fillerMetadata: string }[] = fillingOrders[0].candidateUsers
        .filter(candidate => !candidate.fillerMetadata.equals(PublicKey.default))
        .map(candidate => {
            return {
                loaneeMetadata: candidate.userMetadata.toBase58(),
                fillerMetadata: candidate.fillerMetadata.toBase58()
            };
        });
    expect(actual).to.have.deep.ordered.members(expected)
}
