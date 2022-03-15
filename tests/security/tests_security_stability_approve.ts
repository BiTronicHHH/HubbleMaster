import * as operations_stability from "../operations_stability";
import * as instructions_stability from '../../src/instructions_stability';
import * as utils from "../../src/utils";
import * as chai from 'chai'
import { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { setUpProgram } from "../../src/set_up";
import * as set_up from "../../src/set_up";

chai.use(chaiAsPromised)

describe('tests_security_stability_approve', () => {
    const { initialMarketOwner, provider, program, } = setUpProgram();
    const env = { provider, program, initialMarketOwner } as set_up.Env;

    it('security_stability_approve_same_state_account_twice', async () => {
        const { borrowingAccounts, } = await operations_stability.createMarketAndStabilityPool(env);

        const { keyPair: user } = await utils.solAccountWithMinBalance(provider, 1);

        console.log("user", user.publicKey.toString());
        const userStabilityProviderAccounts = await set_up.setUpStabilityProviderUserAccounts(
            provider,
            [user],
            user.publicKey,
            program,
            borrowingAccounts
        );

        await instructions_stability.approveStability(
            program,
            user.publicKey,
            userStabilityProviderAccounts.stabilityProviderState,
            borrowingAccounts.stabilityPoolState.publicKey,
            [user]
        );
        await expect(instructions_stability.approveStability(
            program,
            user.publicKey,
            userStabilityProviderAccounts.stabilityProviderState,
            borrowingAccounts.stabilityPoolState.publicKey,
            [user]
        )).to.be.rejectedWith("0x0"); // ATA mismatch

    });

});
