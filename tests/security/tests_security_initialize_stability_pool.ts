import { airdropSol } from "../operations_borrowing";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as instructions_stability from '../../src/instructions_stability';
import * as utils from "../../src/utils";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { Keypair, Transaction, TransactionInstruction } from "@solana/web3.js";
import * as set_up from "../../src/set_up";
import { setUpProgram } from "../../src/set_up";
import { displayBorrowingMarketState } from "../../src/utils_display";

chai.use(chaiAsPromised)

describe('tests_security_initialize_stability_pool', () => {
    const { initialMarketOwner, provider, program, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('security_initialize_stability_pool_different_owner_from_borrowing_market', async () => {
        const borrowingAccounts = await set_up.setUpBorrowingGlobalAccounts(
            provider,
            initialMarketOwner,
            program);

        const differentOwner = Keypair.generate();
        await airdropSol(provider, program, 1, differentOwner.publicKey);
        console.log("DIFFERENT OWNER - " + differentOwner.publicKey);

        const differentOwnerStabilityAccounts = await set_up.setUpStabilityPoolAccounts(
            provider,
            program,
            differentOwner.publicKey,
            borrowingAccounts);

        await instructions_borrow
            .initializeBorrowingMarket(
                program,
                initialMarketOwner,
                borrowingAccounts
            );
        await displayBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey)

        // different initial market owner
        const ix = program.instruction.stabilityInitialize({
            accounts: instructions_stability.utils.initializeStabilityPoolAccounts(
                differentOwner.publicKey, // different owner
                borrowingAccounts,
                differentOwnerStabilityAccounts
            ),
            signers: [
                borrowingAccounts.stabilityPoolState,
                differentOwnerStabilityAccounts.stabilityVaults,
                differentOwner
            ]
        });

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, differentOwner.publicKey, [
            borrowingAccounts.stabilityPoolState,
            differentOwnerStabilityAccounts.stabilityVaults,
            differentOwner
        ])).to.be.rejectedWith("0x8d"); // anchor has_one violation
    });

    it('security_initialize_stability_pool_different_stability_accounts_owner', async () => {
        const borrowingAccounts = await set_up.setUpBorrowingGlobalAccounts(
            provider,
            initialMarketOwner,
            program);

        const differentOwner = Keypair.generate();
        await airdropSol(provider, program, 1, differentOwner.publicKey);
        console.log("DIFFERENT OWNER - " + differentOwner.publicKey);

        const differentOwnerStabilityAccounts = await set_up.setUpStabilityPoolAccounts(
            provider,
            program,
            differentOwner.publicKey,
            borrowingAccounts);

        await instructions_borrow
            .initializeBorrowingMarket(
                program,
                initialMarketOwner,
                borrowingAccounts
            );
        await displayBorrowingMarketState(program, borrowingAccounts.borrowingMarketState.publicKey)

        // different stability accounts owner
        const ix: TransactionInstruction = program.instruction.stabilityInitialize({
            accounts: instructions_stability.utils.initializeStabilityPoolAccounts(
                initialMarketOwner,
                borrowingAccounts,
                differentOwnerStabilityAccounts
            ),
            remainingAccounts: [
                {
                    pubkey: differentOwner.publicKey, // add different owner as signer
                    isWritable: true,
                    isSigner: true
                }
            ],
            signers: [
                borrowingAccounts.stabilityPoolState,
                differentOwnerStabilityAccounts.stabilityVaults,
                differentOwner,
            ]
        });

        for (let i = 0; i < ix.keys.length; i++) {
            // mark the initial market owner as a non-signer
            if (ix.keys[i].pubkey.toBase58() === initialMarketOwner.toBase58()) {
                ix.keys[i].isSigner = false;
                ix.keys[i].isWritable = false;
            }
        }

        const tx = new Transaction();
        tx.add(ix);

        await expect(utils.send(provider, tx, differentOwner.publicKey, [
            borrowingAccounts.stabilityPoolState,
            differentOwnerStabilityAccounts.stabilityVaults,
            differentOwner
        ])).to.be.rejectedWith("Cross-program invocation with unauthorized signer or writable account");
    });
});
