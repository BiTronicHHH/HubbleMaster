import * as operations_borrowing from "../operations_borrowing";
import * as instructions_borrow from '../../src/instructions_borrow';
import * as utils from "../../src/utils";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { Keypair } from "@solana/web3.js";
import { Env, setUpAssociatedStablecoinAccount, setUpProgram } from "../../src/set_up";

chai.use(chaiAsPromised)

describe('tests_security_approve_trove', () => {
    const { initialMarketOwner, provider, program, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as Env;

    it('security_approve_trove_not_user_stablecoin_ata', async () => {
        const borrowingGlobalAccounts = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;

        const user1 = (await utils.solAccountWithMinBalance(provider, 1)).keyPair;
        const user2 = (await utils.solAccountWithMinBalance(provider, 1)).keyPair;

        const user2StablecoinAta = await setUpAssociatedStablecoinAccount(
            provider,
            user2.publicKey,
            user2.publicKey,
            borrowingGlobalAccounts.stablecoinMint,
            [user2]
        );

        // user1 specifies user2 stablecoin ATA
        await expect(instructions_borrow
            .initializeTrove(
                program,
                user1.publicKey,
                Keypair.generate(),
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                user2StablecoinAta, // user2 ATA
                [user1])
        ).to.be.rejectedWith("0x44d"); // ATA mismatch
    });

    it('security_approve_trove_different_mint_for_stablecoin_ata', async () => {
        const borrowingGlobalAccounts = (await operations_borrowing.initialiseBorrowingMarkets(env)).borrowingAccounts;

        const user = (await utils.solAccountWithMinBalance(provider, 1)).keyPair;

        const userEthAta = await setUpAssociatedStablecoinAccount(
            provider,
            user.publicKey,
            user.publicKey,
            borrowingGlobalAccounts.ethMint,
            [user]
        );

        // user1 specifies user2 stablecoin ATA
        await expect(instructions_borrow
            .initializeTrove(
                program,
                user.publicKey,
                Keypair.generate(),
                borrowingGlobalAccounts.borrowingMarketState.publicKey,
                userEthAta, // ETH ATA
                [user])
        ).to.be.rejectedWith("0x44d"); // ATA mismatch
    });
});
