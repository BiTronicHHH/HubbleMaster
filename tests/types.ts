import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

export type User = {
    address: PublicKey;
    id: number;
};

export type BorrowingMarketState = {
    numUsers: number;
    stablecoinBorrowed: number;
    depositedCollateral: CollateralAmounts
    inactiveCollateral: CollateralAmounts
    stablecoinMint: PublicKey,
    stablecoinMintAuthority: PublicKey,
    stablecoinMintSeed: number,
    hbbMint: PublicKey,
    hbbMintAuthority: PublicKey,
    hbbMintSeed: number,
    redemptionsQueue: PublicKey,
};

export type UserMetadata = {
    version: number;
    status: number;
    userId: number;
    metadataPk: PublicKey;
    owner: PublicKey;
    borrowingMarketState: PublicKey;
    stablecoinAta: PublicKey;
    depositedCollateral: CollateralAmounts;
    inactiveCollateral: CollateralAmounts;
    borrowedStablecoin: number;
    userStake: number;
    userCollateralRewardPerToken: TokenMap;
    userStablecoinRewardPerToken: number;
};

export type BorrowingVaults = {
    borrowingMarketState: PublicKey;
    burningVault: PublicKey;
    burningVaultAuthority: PublicKey;
    burningVaultSeed: number;
    borrowingFeesVault: PublicKey;
    borrowingFeesVaultAuthority: PublicKey;
    borrowingFeesVaultSeed: number;
    collateralVaultSol: PublicKey;
    collateralVaultSrm: PublicKey;
    collateralVaultEth: PublicKey;
    collateralVaultBtc: PublicKey;
    collateralVaultRay: PublicKey;
    collateralVaultFtt: PublicKey;
    collateralVaultsAuthority: PublicKey;
    collateralVaultSrmSeed: number;
    collateralVaultEthSeed: number;
    collateralVaultBtcSeed: number;
    collateralVaultRaySeed: number;
    collateralVaultFttSeed: number;
    srmMint: PublicKey;
    ethMint: PublicKey;
    btcMint: PublicKey;
    rayMint: PublicKey;
    fttMint: PublicKey;
};

export type GlobalConfig = {
    version: number;
    isBorrowingAllowed: boolean;
    borrowLimitUsdh: number;
}

export type TokenMap = {
    sol: BN,
    btc: BN,
    eth: BN,
    srm: BN,
    ftt: BN,
    ray: BN,
}

export type CollateralAmounts = {
    sol: number,
    btc: number,
    eth: number,
    srm: number,
    ftt: number,
    ray: number,
}

export type RedemptionOrder = {
    id: number,
    status: number;
    lastReset: number;
    redeemerUserMetadata: PublicKey;
    redeemer: PublicKey;
    requestedAmount: number;
    remainingAmount: number;
    redemptionPrices: TokenMap;
    candidateUsers: CandidateRedemptionUser[];
};

export type CandidateRedemptionUser = {
    status: number;
    userId: number;
    userMetadata: PublicKey;
    debt: number;
    collateralRatio: number;
    fillerMetadata: PublicKey;
};

export type StabilityPoolState = {
    borrowingMarketState: PublicKey,
    epochToScaleToSum: PublicKey,
    liquidationsQueue: PublicKey,
    numUsers: number,
    stablecoinDeposited: number
}

export type StabilityVaults = {
    stabilityPoolState: PublicKey,
    stablecoinStabilityPoolVault: PublicKey,
    stablecoinStabilityPoolVaultAuthority: PublicKey,
    stablecoinStabilityPoolVaultSeed: number,
    liquidationRewardsVaultSol: PublicKey,
    liquidationRewardsVaultSrm: PublicKey,
    liquidationRewardsVaultEth: PublicKey,
    liquidationRewardsVaultBtc: PublicKey,
    liquidationRewardsVaultRay: PublicKey,
    liquidationRewardsVaultFtt: PublicKey,
    liquidationRewardsVaultAuthority: PublicKey,
    liquidationRewardsVaultSeed: number,
    srmMint: PublicKey,
    ethMint: PublicKey,
    btcMint: PublicKey,
    rayMint: PublicKey,
    fttMint: PublicKey,
}

export type StabilityProviderState = {
    userId: number,
    depositedStablecoin: number
}

export type StakingPoolState = {
    borrowingMarketState: PublicKey,
    totalDistributedRewards: number;
    rewardsNotYetClaimed: number;
    totalStake: number;
    rewardPerToken: number;
    prevRewardLoss: number,
    stakingVault: PublicKey,
    stakingVaultAuthority: PublicKey,
    stakingVaultSeed: number,
};

export type UserStakingState = {
    rewardsTally: BigInt;
    userStake: number;
    version: number;
    userId: number;
    owner: PublicKey;
    stakingPoolState: PublicKey;
};

export type CollateralToken = "SOL" | "ETH" | "BTC" | "SRM" | "RAY" | "FTT";
export function collateralTokenToNumber(t: CollateralToken): number {
    switch (t) {
        case "SOL": return 0;
        case "ETH": return 1;
        case "BTC": return 2;
        case "SRM": return 3;
        case "RAY": return 4;
        case "FTT": return 5;
        default: return -100;
    }
}
export function numberToCollateralToken(t: number): CollateralToken {
    switch (t) {
        case 0: return "SOL";
        case 1: return "ETH";
        case 2: return "BTC";
        case 3: return "SRM";
        case 4: return "RAY";
        case 5: return "FTT";
        default: return "SOL";
    }
}

export type StabilityToken = "SOL" | "ETH" | "BTC" | "SRM" | "RAY" | "FTT" | "HBB";
export function stabilityTokenToNumber(t: StabilityToken): number {
    switch (t) {
        case "SOL": return 0;
        case "ETH": return 1;
        case "BTC": return 2;
        case "SRM": return 3;
        case "RAY": return 4;
        case "FTT": return 5;
        case "HBB": return 6;
        default: return -100;
    }
}
export function numberToStabilityToken(t: number): StabilityToken {
    switch (t) {
        case 0: return "SOL";
        case 1: return "ETH";
        case 2: return "BTC";
        case 3: return "SRM";
        case 4: return "RAY";
        case 5: return "FTT";
        case 6: return "HBB";
        default: return "SOL";
    }
}