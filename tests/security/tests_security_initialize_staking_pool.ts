import * as anchor from '@project-serum/anchor';
import { airdropSol } from "../operations_borrowing";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as utils from "../../src/utils";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { TokenInstructions } from "@project-serum/serum";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import * as set_up from "../../src/set_up";
import { setUpProgram } from "../../src/set_up";

chai.use(chaiAsPromised)

describe('tests_security_initialize_staking_pool', () => {
    const { initialMarketOwner, provider, program, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('security_initialize_staking_pool_different_owner_from_borrowing_market', async () => {
        const borrowingAccounts = await set_up.setUpBorrowingGlobalAccounts(
            provider,
            initialMarketOwner,
            program);

        const differentOwner = Keypair.generate();
        await airdropSol(provider, program, 1, differentOwner.publicKey);

        const differentOwnerAccounts = await set_up.setUpStakingPoolAccounts(
            provider,
            differentOwner.publicKey,
            program,
            borrowingAccounts);

        await instructions_borrow
            .initializeBorrowingMarket(
                program,
                initialMarketOwner,
                borrowingAccounts
            );

        const treasuryFeeRate = 1500;
        // different initial market owner
        const ix = await program.instruction.stakingInitialize(new anchor.BN(treasuryFeeRate), {
            accounts: {
                initialMarketOwner: differentOwner.publicKey, // different initial market owner
                borrowingMarketState: borrowingAccounts.borrowingMarketState.publicKey,
                stakingPoolState: borrowingAccounts.stakingPoolState.publicKey,
                stakingVault: differentOwnerAccounts.stakingVault,
                treasuryVault: differentOwnerAccounts.treasuryVault,
                tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                systemProgram: anchor.web3.SystemProgram.programId,
            },
            signers: [borrowingAccounts.stakingPoolState, differentOwner]
        });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, differentOwner.publicKey, [borrowingAccounts.stakingPoolState, differentOwner]))
            .to.be.rejectedWith("0x8d"); // anchor has_one violation
    });
});
