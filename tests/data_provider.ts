import { PublicKey } from "@solana/web3.js";
import { BorrowingMarketState, BorrowingVaults, RedemptionOrder, StabilityPoolState, StabilityProviderState, GlobalConfig, StabilityVaults, StakingPoolState, CollateralToken, UserMetadata, UserStakingState, CollateralAmounts } from "./types";
import * as anchor from "@project-serum/anchor";
import * as utils from "../src/utils";
import { BorrowingGlobalAccounts, BorrowingUserState, StakingPoolAccounts } from "../src/set_up";

export async function getBorrowingMarket(
    program: anchor.Program,
    borrowingGlobalAccounts: BorrowingGlobalAccounts
): Promise<BorrowingMarketState> {
    return getBorrowingMarketState(program, borrowingGlobalAccounts.borrowingMarketState.publicKey);
}
export async function getBorrowingMarketState(
    program: anchor.Program,
    account: PublicKey
): Promise<BorrowingMarketState> {
    const borrowingMarketState: any = await program.account.borrowingMarketState.fetch(
        account
    );

    let numUsers = borrowingMarketState.numUsers.toNumber();
    let stablecoinBorrowed = borrowingMarketState.stablecoinBorrowed.toNumber();
    let depositedCollateral = toNumber(borrowingMarketState.depositedCollateral);
    let inactiveCollateral = toNumber(borrowingMarketState.inactiveCollateral);

    return {
        ...borrowingMarketState,
        stablecoinBorrowed,
        numUsers,
        depositedCollateral,
        inactiveCollateral
    };
}

export async function getUserState(
    program: anchor.Program,
    borrowerAccounts: BorrowingUserState
): Promise<UserMetadata> {
    return getUserMetadata(program, borrowerAccounts.borrowerAccounts.userMetadata.publicKey);
}

export async function getUserMetadata(
    program: anchor.Program,
    account: PublicKey
): Promise<UserMetadata> {
    const userMetadata: any = await program.account.userMetadata.fetch(account);

    return {
        version: userMetadata.version,
        status: userMetadata.status,
        userId: userMetadata.userId.toNumber(),
        metadataPk: userMetadata.metadataPk,
        owner: userMetadata.owner,
        borrowingMarketState: userMetadata.borrowingMarketState,
        stablecoinAta: userMetadata.stablecoinAta,
        depositedCollateral: toNumber(userMetadata.depositedCollateral),
        inactiveCollateral: toNumber(userMetadata.inactiveCollateral),
        borrowedStablecoin: userMetadata.borrowedStablecoin.toNumber(),
        userStake: userMetadata.userStake.toNumber(),
        userCollateralRewardPerToken: userMetadata.userCollateralRewardPerToken,
        userStablecoinRewardPerToken: userMetadata.userStablecoinRewardPerToken.toNumber(),
    };
}

export async function getBorrowingVaults(
    program: anchor.Program,
    account: PublicKey
): Promise<BorrowingVaults> {
    const borrowingVaults: any = await program.account.borrowingVaults.fetch(account);
    return {
        ...borrowingVaults,
    }
}

export async function getGlobalConfig(
    program: anchor.Program,
    account: PublicKey
): Promise<GlobalConfig> {
    const globalConfig: any = await program.account.globalConfig.fetch(account);
    return {
        version: globalConfig.version,
        isBorrowingAllowed: globalConfig.isBorrowingAllowed,
        borrowLimitUsdh: globalConfig.borrowLimitUsdh.toNumber()
    }
}

export async function getStabilityProviderAccount(
    program: anchor.Program,
    account: PublicKey): Promise<StabilityProviderState> {

    const acc: any = await program.account.stabilityProviderState.fetch(account);

    let userId = acc.userId.toNumber();
    let depositedStablecoin = acc.depositedStablecoin.toNumber();

    return {
        userId,
        depositedStablecoin
    }
}

export async function getStabilityVaults(
    program: anchor.Program,
    account: PublicKey): Promise<StabilityVaults> {

    const acc: any = await program.account.stabilityVaults.fetch(account);

    return {
        ...acc,
    }
}

export async function getStabilityPoolState(
    program: anchor.Program,
    account: PublicKey
): Promise<StabilityPoolState> {
    const acc: any = await program.account.stabilityPoolState.fetch(account);

    const numUsers = acc.numUsers.toNumber();
    const stablecoinDeposited = acc.stablecoinDeposited.toNumber();

    return {
        ...acc,
        numUsers,
        stablecoinDeposited,
    };
}

export async function getStakingPoolState(
    program: anchor.Program,
    account: PublicKey
): Promise<StakingPoolState> {
    const acc: any = await program.account.stakingPoolState.fetch(account);

    let totalDistributedRewards = acc.totalDistributedRewards.toNumber();
    let rewardsNotYetClaimed = acc.rewardsNotYetClaimed.toNumber();
    let totalStake = acc.totalStake.toNumber();
    let rewardPerToken = acc.rewardPerToken.toNumber();
    let prevRewardLoss = acc.prevRewardLoss.toNumber();

    return {
        ...acc,
        totalDistributedRewards,
        rewardsNotYetClaimed,
        totalStake,
        rewardPerToken,
        prevRewardLoss,
    };
}

export async function getRedemptionsQueueData(
    program: anchor.Program,
    account: PublicKey
): Promise<RedemptionOrder[]> {
    let redemptionsQueue: any = await program.account.redemptionsQueue.fetch(account);
    const orders: any[] = redemptionsQueue.orders;
    return orders.map((raw) => {
        return {
            id: raw.id,
            status: raw.status,
            lastReset: raw.lastReset.toNumber(),
            redeemerUserMetadata: raw.redeemerUserMetadata,
            redeemer: raw.redeemer,
            requestedAmount: raw.requestedAmount.toNumber(),
            remainingAmount: raw.remainingAmount.toNumber(),
            redemptionPrices: raw.redemptionPrices,
            candidateUsers: raw.candidateUsers,
        }
    })
}

export async function getUserStakingStateData(
    program: anchor.Program,
    account: PublicKey
): Promise<UserStakingState> {
    const userStakingState: any = await program.account.userStakingState.fetch(account);

    const userId = userStakingState.userId.toNumber();
    const rewardsTally = BigInt(userStakingState.rewardsTally);
    const userStake = userStakingState.userStake.toNumber();

    return {
        ...userStakingState,
        userId,
        rewardsTally,
        userStake,
    };
}

export async function collateralVaultBalance(
    program: anchor.Program,
    borrowingAccounts: BorrowingGlobalAccounts,
    collateralToken: CollateralToken,
): Promise<number | null> {

    return await getCollateralVaultBalance(program, borrowingAccounts.borrowingVaults.publicKey, collateralToken);
}

export async function feesVaultBalance(
    program: anchor.Program,
    borrowingAccounts: BorrowingGlobalAccounts,
): Promise<number | null> {
    let feesVaultAccount = borrowingAccounts.borrowingFeesVault;
    return await getTokenAccountBalance(program, feesVaultAccount);
}

export async function treasuryVaultBalance(
    program: anchor.Program,
    stakingAccounts: StakingPoolAccounts,
): Promise<number | null> {
    let feesVaultAccount = stakingAccounts.treasuryVault;
    return await getTokenAccountBalance(program, feesVaultAccount);
}

export async function stablecoinBalance(
    program: anchor.Program,
    borrowingAccounts: BorrowingUserState,
): Promise<number | null> {
    return await getTokenAccountBalance(program, borrowingAccounts.borrowerAccounts.stablecoinAta);
}

export async function getCollateralVaultBalance(
    program: anchor.Program,
    borrowingVaultsKey: PublicKey,
    collateralToken: CollateralToken,
): Promise<number | null> {

    const borrowingVaults = await getBorrowingVaults(program, borrowingVaultsKey);

    switch (collateralToken) {
        case "SOL":
            const lamports = (await program.provider.connection.getAccountInfo(borrowingVaults.collateralVaultSol, "confirmed"))?.lamports;
            console.log("Lamports", lamports);
            console.log("Lamports", borrowingVaults.collateralVaultSol.toString());
            const rentExemptionLamports = await program.provider.connection.getMinimumBalanceForRentExemption(9, "confirmed");
            return lamports ? utils.lamportsToColl(lamports - rentExemptionLamports, "SOL") : 0;
        case "BTC":
            return getTokenAccountBalance(program, borrowingVaults.collateralVaultBtc);
        case "ETH":
            return getTokenAccountBalance(program, borrowingVaults.collateralVaultEth);
        case "SRM":
            return getTokenAccountBalance(program, borrowingVaults.collateralVaultSrm);
        case "RAY":
            return getTokenAccountBalance(program, borrowingVaults.collateralVaultRay);
        case "FTT":
            return getTokenAccountBalance(program, borrowingVaults.collateralVaultFtt);
    }
}

export async function getTokenAccountBalance(
    program: anchor.Program,
    tokenAccount: PublicKey,
): Promise<number | null> {
    const tokenAccountBalance = await program.provider.connection.getTokenAccountBalance(tokenAccount);
    return tokenAccountBalance.value.uiAmount;
}

export async function getForcedSolBalanceInLamports(
    provider: anchor.Provider,
    account: PublicKey): Promise<number> {
    let balance = undefined;
    while (balance === undefined) {
        balance = (await provider.connection.getAccountInfo(account))?.lamports;
    }

    return balance;
}

function toNumber(object: any): CollateralAmounts {
    return {
        sol: object.sol.toNumber(),
        eth: object.eth.toNumber(),
        btc: object.btc.toNumber(),
        srm: object.srm.toNumber(),
        ftt: object.ftt.toNumber(),
        ray: object.ray.toNumber(),
    }
}