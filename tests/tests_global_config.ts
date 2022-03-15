import * as anchor from '@project-serum/anchor';
import * as set_up from '../src/set_up';
import { setUpProgram } from "../src/set_up";
import * as instructions_borrow from '../src/instructions_borrow';
import * as assert from "assert";
import { getGlobalConfig } from './data_provider';
import { GlobalConfigOption } from '../src/config';
import { updateGlobalConfig } from '../src/instructions_borrow';
import { expect } from 'chai'
const chai = require('chai')
    .use(require('chai-as-promised'))
import { Keypair } from '@solana/web3.js';


describe('tests_global_config', () => {
    const { initialMarketOwner, provider, program, pyth, } = setUpProgram();

    it('tests_global_config_update_global_config_is_borrowing', async () => {
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
        const globalConfig = await getGlobalConfig(program, borrowingGlobalAccounts.globalConfig.publicKey);
        // Negate the isBorrowingAllowed flag.
        const isBorrowingAllowed = globalConfig.isBorrowingAllowed ? 0 : 1;
        await updateGlobalConfig(program, initialMarketOwner, borrowingGlobalAccounts, GlobalConfigOption.IsBorrowingAllowed, isBorrowingAllowed);
        const globalConfigModified = await getGlobalConfig(program, borrowingGlobalAccounts.globalConfig.publicKey);
        assert.notStrictEqual(globalConfig.isBorrowingAllowed, isBorrowingAllowed);
    });

    it('tests_global_config_update_global_config_borrow_limit_usdh', async () => {
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
        const globalConfig = await getGlobalConfig(program, borrowingGlobalAccounts.globalConfig.publicKey);
        // Increase the borrow limit before we change it.
        const borrowLimitUsdh = Number.parseInt(globalConfig.borrowLimitUsdh.toString()) + 1000
        await updateGlobalConfig(program, initialMarketOwner, borrowingGlobalAccounts, GlobalConfigOption.BorrowLimitUsdh, borrowLimitUsdh);
        const globalConfigModified = await getGlobalConfig(program, borrowingGlobalAccounts.globalConfig.publicKey);
        assert.notStrictEqual(globalConfigModified.borrowLimitUsdh.toString(), borrowLimitUsdh)
    });

    it('tests_global_config_update_global_config_failed', async () => {
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
        const largestKey = (1 << 16) - 1;
        await expect(
            program.rpc.updateGlobalConfig(new anchor.BN(largestKey), new anchor.BN(0), {
                accounts: {
                    initialMarketOwner,
                    globalConfig: borrowingGlobalAccounts.globalConfig.publicKey,
                    systemProgram: anchor.web3.SystemProgram.programId,
                },
            })
        ).to.be.rejectedWith("");
    });

    it('tests_global_config_update_global_config_malicious_signer', async () => {
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
        const globalConfig = await getGlobalConfig(program, borrowingGlobalAccounts.globalConfig.publicKey);
        // Increase the borrow limit before we change it.
        const borrowLimitUsdh = Number.parseInt(globalConfig.borrowLimitUsdh.toString()) + 1000
        const maliciousSigner = new Keypair();

        await expect(
            updateGlobalConfig(program, maliciousSigner.publicKey, borrowingGlobalAccounts, GlobalConfigOption.BorrowLimitUsdh, borrowLimitUsdh)
        ).to.be.rejectedWith("Signature verification failed");
        const globalConfigModified = await getGlobalConfig(program, borrowingGlobalAccounts.globalConfig.publicKey);
        assert.notStrictEqual(globalConfigModified.borrowLimitUsdh.toString(), borrowLimitUsdh)
    });
});