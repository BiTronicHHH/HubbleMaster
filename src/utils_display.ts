import { BorrowingGlobalAccounts, BorrowingUserAccounts, UserAtas } from "./set_up";
import * as anchor from '@project-serum/anchor';
import * as utils from '../src/utils';
import { Keypair, PublicKey } from "@solana/web3.js";
import { UserMetadata } from "../tests/types";

export function printBorrowingMarketState(account: any): string {
    return `BorrowingMarketState {
    version: ${account.version},
    initialMarketOwner: ${account.initialMarketOwner},
    defaultUserMetadata: ${account.defaultUserMetadata}
    stablecoinMint: ${account.stablecoinMint},
    stablecoinMintAuthority: ${account.stablecoinMintAuthority},
    hbbMint: ${account.hbbMint},
    redemptionSortAccounts: ${account.redemptionSortAccounts},
    stablecoinBorrowed: ${account.stablecoinBorrowed},
    depositedCollateral: ${tokenMapToString(account.depositedCollateral)},
    numUsers: ${account.numUsers},
}`;
}

export function printBorrowingVaults(account: any): string {
    return `BorrowingVaults {
    burningVault: ${account.burningVault},
    burningVaultAuthority: ${account.burningVaultAuthority},
    burningVaultSeed: ${account.burningVaultSeed},
    borrowingFeesVault: ${account.borrowingFeesVault},
    borrowingFeesVaultAuthority: ${account.borrowingFeesVaultAuthority},
    borrowingFeesVaultSeed: ${account.borrowingFeesVaultSeed},
    collateralVaultSol: ${account.collateralVaultSol},
    collateralVaultSrm: ${account.collateralVaultSrm},
    collateralVaultEth: ${account.collateralVaultEth},
    collateralVaultBtc: ${account.collateralVaultBtc},
    collateralVaultRay: ${account.collateralVaultRay},
    collateralVaultFtt: ${account.collateralVaultFtt},
    collateralVaultsAuthority: ${account.collateralVaultsAuthority},
    collateralVaultsSeed: ${account.collateralVaultsSeed},
    srmMint: ${account.srmMint},
    ethMint: ${account.ethMint},
    btcMint: ${account.btcMint},
    rayMint: ${account.rayMint},
    fttMint: ${account.fttMint},
}`;
}
export function printStabilityVaults(account: any): string {
    return `StabilityVaults {
        stabilityPoolState: ${account.stabilityPoolState},
        hbbEmissionRewardsVault: ${account.hbbEmissionRewardsVault},
        hbbEmissionRewardsVaultAuthority: ${account.hbbEmissionRewardsVaultAuthority},
        hbbEmissionRewardsVaultSeed: ${account.hbbEmissionRewardsVaultSeed},
        stablecoinStabilityPoolVault: ${account.stablecoinStabilityPoolVault},
        stablecoinStabilityPoolVaultAuthority: ${account.stablecoinStabilityPoolVaultAuthority},
        stablecoinStabilityPoolVaultSeed: ${account.stablecoinStabilityPoolVaultSeed},
        liquidationRewardsVaultSol: ${account.liquidationRewardsVaultSol},
        liquidationRewardsVaultSrm: ${account.liquidationRewardsVaultSrm},
        liquidationRewardsVaultEth: ${account.liquidationRewardsVaultEth},
        liquidationRewardsVaultBtc: ${account.liquidationRewardsVaultBtc},
        liquidationRewardsVaultRay: ${account.liquidationRewardsVaultRay},
        liquidationRewardsVaultFtt: ${account.liquidationRewardsVaultFtt},
        liquidationRewardsVaultAuthority: ${account.liquidationRewardsVaultAuthority},
        liquidationRewardsVaultSeed: ${account.liquidationRewardsVaultSeed},
        srmMint: ${account.srmMint},
        ethMint: ${account.ethMint},
        btcMint: ${account.btcMint},
        rayMint: ${account.rayMint},
        fttMint: ${account.fttMint},
}`;

}

export function printStabilityPoolState(account: any): string {
    return `StabilityPoolState {
    version: ${account.version},
    numUsers: ${account.numUsers},
    totalUsersProvidingStability: ${account.totalUsersProvidingStability},
    hbbEmissionsStartTs: ${account.hbbEmissionsStartTs},
    totalUsdDeposits: ${account.totalUsdDeposits},
    cumulativeGainsTotal: ${stabilityTokenMapToString(account.cumulativeGainsTotal)},
    pendingCollateralGains: ${stabilityTokenMapToString(account.pendingCollateralGains)},
    currentEpoch: ${account.currentEpoch},
    currentScale: ${account.currentScale},
    p: ${account.p},
    lastUsdLossErrorOffset: ${account.lastUsdLossErrorOffset},
    lastCollLossErrorOffset: ${stabilityTokenMapToString(account.lastCollLossErrorOffset)},
    borrowingMarketState: ${account.borrowingMarketState.toString()},
    liquidationQueue: ${account.liquidationsQueue.toString()},
    epochToScaleSum: ${account.epochToScaleToSum.toString()},
}`;
}

export function printStakingPoolState(account: any): string {
    return `StakingPoolState {
        borrowingMarketState: ${account.borrowingMarketState},
        totalDistributedRewards: ${account.totalDistributedRewards},
        rewardsNotYetClaimed: ${account.rewardsNotYetClaimed},
        totalStake: ${account.totalStake},
        rewardPerToken: ${account.rewardPerToken},
        stakingVault: ${account.stakingVault},
    }`;
}

export function printUserStakingState(account: any): string {
    return `UserStakingState {
        rewards_tally: ${account.rewardsTally},
        user_stake: ${account.userStake},
      }`;
}

export function displayTroveDataAccount(account: any): string {
    return `TroveData {
    version: ${account.version},
    owner: ${account.owner},
    userId: ${account.userId},
}`;
}

export function displayUserMetadata(account: UserMetadata) {
    let s = `UserPosition {
    status: ${account.status},
    userId: ${account.userId},
    metadataPk: ${account.metadataPk},
    owner: ${account.owner},
    depositedCollateral: ${tokenMapToString(account.depositedCollateral)},
    borrowedStablecoin: ${account.borrowedStablecoin},
    userStake: ${account.userStake},
    userCollateralRewardPerToken: ${account.userCollateralRewardPerToken},
    userStablecoinRewardPerToken: ${account.userStablecoinRewardPerToken},
}`;
    console.log(s);
}

export function printStabilityProviderState(account: any): string {
    return `StabilityProviderState {
    version: ${account.version},
    owner: ${account.owner},
    userId: ${account.userId},
    userUsdDeposits: ${account.userUsdDeposits},
    userDepositSnapshot: ${depositSnapshotToString(account.userDepositSnapshot)},
    cumulativeGainsPerUser: ${stabilityTokenMapToString(account.cumulativeGainsPerUser)},
    pendingGainsPerUser: ${stabilityTokenMapToString(account.pendingGainsPerUser)},
}`;
}

export function displayGlobalStakingStateAccount(account: any): string {
    return `GlobalStakingState {
    reward_index_begin: ${account.rewardIndexBegin},
    reward_index_last: ${account.rewardIndexLast},
    max_supply: ${account.maxSupply},
    issue_total: ${account.issueTotal},
    issue_last_timestamp: ${account.issueLastTimestamp},
    global_total_staked_amount: ${account.globalTotalStakedAmount},
    global_total_staked_users: ${account.globalTotalStakedUsers},
    rewards_pot: ${account.rewardsPot},
    stake_vault: ${account.stakeVault}
}`;
}

export function displayUserStakingStateAccount(account: any): string {
    return `UserStakingState {
    user_total_staked_amount: ${account.userTotalStakedAmount},
    last_collected_reward_index: ${account.lastCollectedRewardIndex},
    pending_amount: ${account.pendingAmount}
}`;
}

export function displayRewardStateAccount(account: any): string {
    return `Reward {
    amount: ${account.amount},
    timestamp: ${account.timestamp},
    index: ${account.index},
    snapshot_total_staked_amount: ${account.snapshotTotalStakedAmount},
    snapshot_total_staked_users: ${account.snapshotTotalStakedUsers},
    total_claimed_amount: ${account.totalClaimedAmount},
    total_claimed_users: ${account.totalClaimedUsers}
}`;
}

export async function displayTrove(
    program: anchor.Program,
    userAccounts: BorrowingUserAccounts) {
    const troveDataAccount = await program.account.userMetadata.fetch(
        userAccounts.userMetadata.publicKey);
    console.log(`Trove at ${userAccounts.userMetadata.publicKey.toString()} \n${displayTroveDataAccount(troveDataAccount)}`);
}

export async function displayData(
    program: anchor.Program,
    provider: anchor.Provider,
    userAccounts: BorrowingUserAccounts,
    globalAccounts: BorrowingGlobalAccounts,
    user: Keypair) {
    await displayTrove(program, userAccounts);
    await displayBorrowingMarketState(program, globalAccounts.borrowingMarketState.publicKey);
    await displayUserBalances(provider, user.publicKey, globalAccounts, userAccounts);
}

export async function displayBorrowingMarketState(program: anchor.Program, borrowingMarketState: PublicKey) {
    const acc = await program.account.borrowingMarketState.fetch(borrowingMarketState);
    console.log(`${printBorrowingMarketState(acc)}`);
}

export async function displayBorrowingVaults(program: anchor.Program, borrowingVaults: PublicKey) {
    const acc = await program.account.borrowingVaults.fetch(borrowingVaults);
    console.log(`${printBorrowingVaults(acc)}`);
}

export async function displayStabilityVaults(program: anchor.Program, stabilityVaults: PublicKey) {
    const acc = await program.account.stabilityVaults.fetch(stabilityVaults);
    console.log(`${printStabilityVaults(acc)}`);
}

export async function displayUserBalances(
    provider: anchor.Provider,
    user: PublicKey,
    borrowingGlobalAccounts: BorrowingGlobalAccounts,
    userAtas: UserAtas) {

    let stablecoinAccountBalance = await provider.connection.getTokenAccountBalance(userAtas.stablecoinAta);
    let ethAccountBalance = await provider.connection.getTokenAccountBalance(userAtas.ethAta);
    let btcAccountBalance = await provider.connection.getTokenAccountBalance(userAtas.btcAta);
    let srmAccountBalance = await provider.connection.getTokenAccountBalance(userAtas.srmAta);
    let rayAccountBalance = await provider.connection.getTokenAccountBalance(userAtas.rayAta);
    let fttAccountBalance = await provider.connection.getTokenAccountBalance(userAtas.fttAta);

    let solAccount = await provider.connection.getAccountInfo(user);

    console.log(`Balances user -> ${user}`);
    console.log(`   Stablecoin -> ${stablecoinAccountBalance.value.uiAmountString}`);
    console.log(`   SOL -> ${utils.lamportsToColl(solAccount?.lamports, "SOL")}`);
    console.log(`   ETH -> ${ethAccountBalance.value.uiAmountString}`);
    console.log(`   BTC -> ${btcAccountBalance.value.uiAmountString}`);
    console.log(`   SRM -> ${srmAccountBalance.value.uiAmountString}`);
    console.log(`   RAY -> ${rayAccountBalance.value.uiAmountString}`);
    console.log(`   FTT -> ${fttAccountBalance.value.uiAmountString}`);
}

export async function displayStabilityPoolState(program: anchor.Program, stabilityPoolState: PublicKey) {
    const global_data_account = await program.account.stabilityPoolState.fetch(
        stabilityPoolState);
    console.log(`${printStabilityPoolState(global_data_account)}`);

}

export async function displayStakingPoolState(program: anchor.Program, stakingPoolState: PublicKey) {
    const global_data_account = await program.account.stakingPoolState.fetch(
        stakingPoolState
    );
    console.log(`${printStakingPoolState(global_data_account)}`);
}

export async function displayUserStakingPoolStateAccount(program: anchor.Program, userStakingState: PublicKey) {
    const global_data_account = await program.account.userStakingState.fetch(
        userStakingState
    );
    console.log(`${printUserStakingState(global_data_account)}`);
}

function tokenMapToString(tokenMap: any) {
    return `TokenMap { sol: ${new anchor.BN(tokenMap.sol)}, eth: ${new anchor.BN(tokenMap.eth)}, btc: ${new anchor.BN(tokenMap.btc)}, srm: ${new anchor.BN(tokenMap.srm)}, ray: ${new anchor.BN(tokenMap.ray)}, ftt: ${new anchor.BN(tokenMap.ftt)} }`;
}

function stabilityTokenMapToString(tokenMap: any) {
    return `TokenMap { sol: ${new anchor.BN(tokenMap.sol)}, eth: ${new anchor.BN(tokenMap.eth)}, btc: ${new anchor.BN(tokenMap.btc)}, srm: ${new anchor.BN(tokenMap.srm)}, ray: ${new anchor.BN(tokenMap.ray)}, ftt: ${new anchor.BN(tokenMap.ftt)}, hbb: ${new anchor.BN(tokenMap.hbb)}}`;
}

function depositSnapshotToString(depositSnapshot: any) {
    return `DepositSnapshot { sum: ${stabilityTokenMapToString(depositSnapshot.sum)}, product: ${new anchor.BN(depositSnapshot.product)}, scale: ${new anchor.BN(depositSnapshot.scale)}, epoch: ${new anchor.BN(depositSnapshot.epoch)} }`;
}

export async function displayStabilityProviderState(program: anchor.Program, stabilityProviderState: PublicKey) {
    const global_data_account = await program.account.stabilityProviderState.fetch(
        stabilityProviderState);
    console.log(`${printStabilityProviderState(global_data_account)}`);
}
