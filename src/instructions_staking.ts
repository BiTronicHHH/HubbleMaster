import * as anchor from '@project-serum/anchor';
import { Keypair, PublicKey, Signer } from "@solana/web3.js";
import { TokenInstructions } from "@project-serum/serum";
import { getBorrowingVaults, getStakingPoolState } from "../tests/data_provider";

export async function initializeStakingPool(
    program: anchor.Program,
    initialMarketOwner: PublicKey,
    borrowingMarketState: PublicKey,
    stakingPoolState: Keypair,
    stakingVault: PublicKey,
    treasuryVault: PublicKey,
    treasuryFeeRate: number
) {

    const tx = await program.rpc.stakingInitialize(new anchor.BN(treasuryFeeRate),
        {
            accounts: {
                initialMarketOwner,
                borrowingMarketState,
                stakingPoolState: stakingPoolState.publicKey,
                stakingVault,
                treasuryVault,
                tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                systemProgram: anchor.web3.SystemProgram.programId,
            },
            signers: [stakingPoolState]
        });
    console.log('initializeStakingPool done signature:', tx);
}


export async function approveStakingPool(
    program: anchor.Program,
    owner: PublicKey,
    userStakingState: Keypair,
    stakingPoolState: PublicKey,
    signers: Array<Signer>
) {
    console.log("user", owner.toString());
    console.log("userStakingState", userStakingState.toString());
    console.log("stakingPoolState", stakingPoolState.toString());

    const tx = await program.rpc.stakingApprove({
        accounts: {
            owner,
            userStakingState: userStakingState.publicKey,
            stakingPoolState,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers: [...signers, userStakingState]
    });

    console.log("approveStaking done signature:", tx);
}

export async function stake(
    program: anchor.Program,
    owner: PublicKey,
    userStakingState: PublicKey,
    borrowingMarketState: PublicKey,
    stakingPoolState: PublicKey,
    stakingVault: PublicKey,
    userHbbStakingAta: PublicKey,
    signers: Array<Signer>,
    amount: number
) {
    console.log("Staking HBB");
    const tx = await program.rpc.stakingStakeHbb(new anchor.BN(amount), {
        accounts: {
            owner,
            userStakingState,
            borrowingMarketState,
            stakingPoolState,
            stakingVault,
            userHbbStakingAta,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
        },
        signers,
    });

    console.log("Staking done signature:", tx);
}

export async function unstake(
    program: anchor.Program,
    owner: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    stakingPoolState: PublicKey,
    userStakingState: PublicKey,
    userHbbStakingAta: PublicKey,
    userStablecoinRewardsAta: PublicKey,
    stakingVault: PublicKey,
    borrowingFeesVault: PublicKey,
    signers: Array<Signer>,
    amount: number
) {
    const stakingVaultAuthority = (await getStakingPoolState(program, stakingPoolState)).stakingVaultAuthority;
    const borrowingFeesVaultAuthority = (await getBorrowingVaults(program, borrowingVaults)).borrowingFeesVaultAuthority;

    const tx = await program.rpc.unstakeHbb(new anchor.BN(amount), {
        accounts: {
            owner,
            borrowingMarketState,
            borrowingVaults,
            stakingPoolState,
            userStakingState,
            borrowingFeesVault,
            borrowingFeesVaultAuthority,
            userStablecoinRewardsAta,
            stakingVault,
            stakingVaultAuthority,
            userHbbStakingAta,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers,
    });

    console.log("Unstake done signature:", tx);
}

export async function harvestReward(
    program: anchor.Program,
    owner: PublicKey,
    borrowingMarketState: PublicKey,
    borrowingVaults: PublicKey,
    stakingPoolState: PublicKey,
    userStakingState: PublicKey,
    userStablecoinRewardsAta: PublicKey,
    borrowingFeesVault: PublicKey,
    signers: Array<Signer>
) {
    const borrowingFeesVaultAuthority = (await getBorrowingVaults(program, borrowingVaults)).borrowingFeesVaultAuthority;

    const tx = await program.rpc.stakingHarvestReward({
        accounts: {
            borrowingMarketState,
            borrowingVaults,
            stakingPoolState,
            userStakingState,
            owner,
            borrowingFeesVault,
            borrowingFeesVaultAuthority,
            userStablecoinRewardsAta,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers,
    });

    console.log("harvestReward done signature", tx);
}
