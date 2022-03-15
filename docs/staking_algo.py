class PullBasedDistribution:
    "Constant Time Reward Distribution with Changing Stake Sizes"

    def __init__(self):
        # meta info, not needed for logic
        self.total_distributed_rewards = 0
        self.rewards_not_yet_claimed = 0

        # logic for the contract
        self.total_stake = 0
        self.reward_per_token = 0
        self.stake = {}
        self.reward_tally = {}

    def print_state(self):
        print("Current state")
        print(f"   total_stake = {self.total_stake}")
        print(f"   reward_per_token = {self.reward_per_token}")
        print(f"   stake = {self.stake}")
        print(f"   reward_tally = {self.reward_tally}")
        print(f"   ")
        print(
            f"   total_distributed_rewards = {self.total_distributed_rewards}")
        print(f"   rewards_not_yet_claimed = {self.rewards_not_yet_claimed}")
        print("")

    def deposit_stake(self, address, amount):
        "Increase the stake of `address` by `amount`"
        print(f"Depositing {address} {amount}")
        if address not in self.stake:
            self.stake[address] = 0
            self.reward_tally[address] = 0

        self.stake[address] = self.stake[address] + amount
        self.reward_tally[address] = self.reward_tally[address] + \
            self.reward_per_token * amount
        self.total_stake = self.total_stake + amount

    def distribute(self, reward):
        "Distribute `reward` proportionally to active stakes"
        print(f"Distributing {reward}")

        self.total_distributed_rewards += reward
        self.rewards_not_yet_claimed += reward

        if self.total_stake == 0:
            raise Exception("Cannot distribute to staking pool with 0 stake")

        self.reward_per_token = self.reward_per_token + reward / self.total_stake

    def _compute_reward(self, address):
        "Compute reward of `address`"
        print(f"self.stake[address] {self.stake[address]}")
        print(f"self.reward_tally[address] {self.reward_tally[address]}")

        return self.stake[address] * self.reward_per_token - self.reward_tally[address]

    def withdraw_stake(self, address, amount):
        "Decrease the stake of `address` by `amount`"
        if address not in self.stake:
            raise Exception("Stake not found for given address")

        if amount > self.stake[address]:
            raise Exception("Requested amount greater than staked amount")

        print(f"self.stake[address] {self.stake[address]}")
        self.stake[address] = self.stake[address] - amount
        print(f"self.stake[address] {self.stake[address]}")
        self.reward_tally[address] = self.reward_tally[address] - \
            self.reward_per_token * amount
        self.total_stake = self.total_stake - amount
        return amount

    def withdraw_reward(self, address):
        "Withdraw rewards of `address`"
        reward = self._compute_reward(address)
        self.reward_tally[address] = self.stake[address] * \
            self.reward_per_token
        print(f"Withdrawing {reward} to {address}")
        return reward

    def accumulated_rewards(self, address):
        return self._compute_reward(address)


def assert_fuzzy_eq(left, right):
    assert abs(left - right) < 0.001

def test_one_user():
    '''
    deposit, distribute, deposit iar, distribute == assert

    '''
    addr1 = "user_one"
    addr2 = "user_two"

    contract = PullBasedDistribution()

    contract.deposit_stake(addr1, 100)
    assert contract.accumulated_rewards(addr1) == 0

    contract.distribute(10)
    assert contract.accumulated_rewards(addr1) == 10

    contract.distribute(15)
    assert contract.accumulated_rewards(addr1) == 25

    contract.deposit_stake(addr1, 100)
    assert contract.accumulated_rewards(addr1) == 25

    contract.distribute(15)
    assert contract.accumulated_rewards(addr1) == 40


def test_two_users():
    '''
    deposit, distribute, deposit iar, distribute == assert

    '''
    addr1 = "user_one"
    addr2 = "user_two"

    contract = PullBasedDistribution()

    contract.deposit_stake(addr1, 100)
    assert contract.accumulated_rewards(addr1) == 0

    contract.distribute(10)
    assert contract.accumulated_rewards(addr1) == 10

    contract.deposit_stake(addr2, 100)
    contract.distribute(15)
    assert contract.accumulated_rewards(addr1) == (10 + 15 / 2)

    contract.deposit_stake(addr1, 100)
    assert contract.accumulated_rewards(addr1) == (10 + 15 / 2)

    contract.distribute(15)
    print(f"user one {contract.accumulated_rewards(addr1)}")
    print(f"user one {(10 + 15/2 + 15 * 2/3)}")
    assert_fuzzy_eq(contract.accumulated_rewards(
        addr1), (10 + 15 / 2 + 15 * 2 / 3))


def test_two_users_deposit():
    '''
    deposit, distribute, deposit iar, distribute == assert

    '''
    addr1 = "user_one"
    addr2 = "user_two"

    contract = PullBasedDistribution()

    contract.deposit_stake(addr1, 100)
    contract.deposit_stake(addr2, 100)
    assert contract.accumulated_rewards(addr1) == 0
    assert contract.accumulated_rewards(addr2) == 0

    contract.distribute(10)             # 5
    contract.deposit_stake(addr1, 100)  # 2/3
    contract.distribute(10)             # 2/3 * 10

    print(f"{contract.accumulated_rewards(addr1)} == {(5 + 2 / 3 * 10)}")
    # assert contract.accumulated_rewards(addr1) == (5 + 2 / 3 * 10)

    contract.withdraw_stake(addr1, 200)
    print(f"{contract.accumulated_rewards(addr1)} == {(5 + 2 / 3 * 10)}")


def test_staking_harvest_single():
    addr1 = "user_one"

    contract = PullBasedDistribution()
    contract.deposit_stake(addr1, 100000000)
    contract.distribute(500000)

    contract.print_state()
    

def main():
    # test_one_user()
    # test_two_users()
    # test_two_users_deposit()
    test_staking_harvest_single()


main()
