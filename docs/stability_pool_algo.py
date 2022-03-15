# https://github.com/liquity/dev/blob/main/papers/Scalable_Reward_Distribution_with_Compounding_Stakes.pdf

from dataclasses import dataclass
from typing import Dict, Tuple
from enum import Enum
from copy import copy

DECIMAL_PRECISION = 10 ** 18
SCALE_FACTOR = 10 ** 9


class Coin(Enum):
    ETH = 0,
    SOL = 1,
    BTC = 2,
    SRM = 3


@dataclass
class Snapshot:
    S: Dict[Coin, int]
    P: int
    scale: int
    epoch: int


def default_coin_map(default_value: 0) -> Dict[Coin, int]:
    return {
        Coin.ETH: default_value,
        Coin.SOL: default_value,
        Coin.BTC: default_value,
        Coin.SRM: default_value,
    }


def sol_to_lamports(sol):
    LAMPORTS_PER_SOL = 1_000_000_000
    return sol * LAMPORTS_PER_SOL


def stablecoin_decimal_to_u64(amount):
    STABLECOIN_FACTOR = 10_000.0
    return amount * STABLECOIN_FACTOR


class SP:
    def __init__(self):

        # Global data

        self.total_usd_deposits = 0
        # Current epoch is the state of the liquidation pool,
        # it counts the number of times when the pool was emptied,
        # everytime we have a full stability pool depletion current_epoch += 1
        self.current_epoch = 0
        self.P = DECIMAL_PRECISION
        self.current_scale = 0

        # Per user data

        self.user_usd_deposits: Dict[str, int] = {}
        self.user_deposit_snapshots: Dict[str, Snapshot] = {}

        # Data necessary to calculate the rewards & usd deposits
        # in O(1)

        # Running product of the stability pool state. The user registers
        # a snapshot of thie P and when it withdraws it compares with the
        # latest running P, the difference reflects the number of liquidations
        # that happened between now and the snapshot

        self.epoch_to_scale_to_sum: Dict[int, Dict[int, Dict[Coin, int]]] = {
            0: {
                0: default_coin_map(0.0)
            }
        }

        # Liquidation vault
        self.collateral_rewards_vault: Dict[Coin, int] = default_coin_map(0.0)

        # Metadata for testing and reporting
        self.cumulative_gains_per_user: Dict[str, Dict[Coin, int]] = {}
        self.cumulative_gains_total: Dict[Coin, int] = default_coin_map(0.0)

    def approve_depositing(self, user):
        self.cumulative_gains_per_user[user] = default_coin_map(0.0)
        self.user_deposit_snapshots[user] = None
        self.user_usd_deposits[user] = 0

    def deposit(self, user, amount):
        self.harvest(user)
        self.send_usd_to_stability_pool(user, amount)
        self.update_deposit_and_snapshot(
            user,
            self.get_compounded_usd_deposit(user) + amount)

    def withdraw(self, user, amount):
        initial_deposit = self.user_usd_deposits[user]
        if initial_deposit == 0:
            return

        self.harvest(user)
        compounded_usd_deposit = self.get_compounded_usd_deposit(user)
        usd_to_withdraw = min(compounded_usd_deposit, amount)

        self.send_usd_to_depositor(user, usd_to_withdraw)
        self.update_deposit_and_snapshot(
            user,
            compounded_usd_deposit - usd_to_withdraw)

    def harvest(self, user):
        for coin in [Coin.ETH, Coin.SOL, Coin.SRM, Coin.BTC]:
            pending_gain = self.get_depositor_pending_gain(user, coin)
            self.send_pending_gain_to_depositor(user, coin, pending_gain)

    def liquidate(
        self,
        debt_to_offset: int,
        coll_to_add: Dict[Coin, int]
    ):

        total_lusd = self.total_usd_deposits
        if total_lusd == 0 or debt_to_offset == 0:
            return

        (coll_gain_per_unit_staked,
         usd_loss_per_unit_staked) = self.compute_rewards_per_unit_staked(
            coll_to_add,
            debt_to_offset,
            total_lusd)

        self.update_reward_sum_and_product(
            coll_gain_per_unit_staked,
            usd_loss_per_unit_staked
        )

        self.move_offset_coll_and_debt(coll_to_add, debt_to_offset)

    def get_depositor_pending_gain(self, user: str, asset: Coin):
        initial_deposit = self.user_usd_deposits[user]

        if initial_deposit == 0:
            return 0

        deposit_snapshot = self.user_deposit_snapshots[user]
        pending_gain = self.get_pending_gain_from_snapshots(
            initial_deposit,
            deposit_snapshot,
            asset)

        return pending_gain

    def get_pending_gain_from_snapshots(
            self,
            initial_deposit: int,
            deposit_snapshot: Snapshot,
            asset: Coin):

        epoch_snapshot = deposit_snapshot.epoch
        scale_snapshot = deposit_snapshot.scale

        S_snapshot = deposit_snapshot.S
        P_snapshot = deposit_snapshot.P

        first_portion = self.epoch_to_scale_to_sum[epoch_snapshot][scale_snapshot][asset] - S_snapshot[asset]
        second_portion = self.epoch_to_scale_to_sum[epoch_snapshot].get(
            scale_snapshot + 1, default_coin_map(0.0)).get(asset, 0) / SCALE_FACTOR

        gain = (initial_deposit * (first_portion +
                                   second_portion) / P_snapshot) / DECIMAL_PRECISION

        return gain

    def get_compounded_usd_deposit(self, user):
        # This function calculates the current USD deposit
        # of a user by taking into account the initial deposit
        # and subsequent liquidations

        initial_deposit = self.user_usd_deposits[user]
        deposit_snapshot = self.user_deposit_snapshots[user]

        if initial_deposit == 0 or deposit_snapshot is None:
            return 0

        compounded_deposit = self.get_compounded_stake_from_snapshots(
            initial_deposit, deposit_snapshot)

        return compounded_deposit

    def get_compounded_stake_from_snapshots(self, initial_stake, snapshot: Snapshot):

        snapshot_P = snapshot.P
        scale_snapshot = snapshot.scale
        epoch_snapshot = snapshot.epoch

        if epoch_snapshot < self.current_epoch:
            return 0

        compounded_stake = 0
        scale_diff = self.current_scale - scale_snapshot

        if scale_diff == 0:
            compounded_stake = initial_stake * self.P / snapshot_P

        elif scale_diff == 1:
            compounded_stake = initial_stake * self.P / snapshot_P / SCALE_FACTOR

        else:
            compounded_stake = 0

        return compounded_stake

    def send_usd_to_stability_pool(self, user, amount):
        self.total_usd_deposits += amount

    def send_usd_to_depositor(self, user, amount):
        self.total_usd_deposits -= amount

    def update_deposit_and_snapshot(self, user, new_value):
        self.user_usd_deposits[user] = new_value

        if new_value == 0:
            self.user_deposit_snapshots[user] = None
            return

        self.user_deposit_snapshots[user] = Snapshot(
            copy(
                self.epoch_to_scale_to_sum[self.current_epoch][self.current_scale]),
            self.P,
            self.current_scale,
            self.current_epoch)

    def send_pending_gain_to_depositor(self, user, asset, amount):
        # Also make a transfer from the collateral gains vault
        # to the user gains
        self.collateral_rewards_vault[asset] -= amount
        self.cumulative_gains_per_user[user][asset] += amount

    def compute_rewards_per_unit_staked(
            self,
            coll_to_add: Dict[Coin, int],
            debt_to_offset: int,
            total_usd_deposits: int) -> Tuple[Dict[Coin, int], int]:

        # Calculate usd lost (debt absorbed)
        if debt_to_offset == total_usd_deposits:
            usd_loss_per_unit_staked = DECIMAL_PRECISION
        else:
            lusd_loss_numerator = debt_to_offset * DECIMAL_PRECISION
            usd_loss_per_unit_staked = (
                lusd_loss_numerator / total_usd_deposits) + 1

        # Calculate collateral gained
        coll_gained_per_unit_staked: Dict[Coin, int] = {}
        for (asset, amount) in coll_to_add.items():
            coll_numerator = amount * DECIMAL_PRECISION
            coll_gained_per_unit_staked[asset] = coll_numerator / \
                total_usd_deposits

        return (
            coll_gained_per_unit_staked,
            usd_loss_per_unit_staked
        )

    def update_reward_sum_and_product(
        self,
        coll_gained_per_unit_staked: Dict[Coin, int],
        usd_loss_per_unit_staked: int
    ):
        current_P = self.P
        new_P = None

        new_product_factor = DECIMAL_PRECISION - usd_loss_per_unit_staked
        current_scale_cached = self.current_scale
        current_epoch_cached = self.current_epoch
        current_S = self.epoch_to_scale_to_sum[current_epoch_cached][current_scale_cached]

        # Calculate the new S first, before we update P.
        for (asset, amount_gain_per_unit_staked) in coll_gained_per_unit_staked.items():
            marginal_coll_gain = amount_gain_per_unit_staked * current_P
            new_S = current_S[asset] + marginal_coll_gain
            self.epoch_to_scale_to_sum[current_epoch_cached][current_scale_cached][asset] = new_S

        if new_product_factor == 0:
            self.current_epoch += 1
            self.current_scale = 0
            new_P = DECIMAL_PRECISION
        elif current_P * new_product_factor / DECIMAL_PRECISION < SCALE_FACTOR:
            new_P = current_P * new_product_factor * SCALE_FACTOR / DECIMAL_PRECISION
            self.current_scale += 1

        else:
            new_P = current_P * new_product_factor / DECIMAL_PRECISION

        self.P = new_P

    def move_offset_coll_and_debt(
            self,
            coll_to_add: Dict[Coin, int],
            debt_to_offset: int):

        self.total_usd_deposits -= debt_to_offset
        for (asset, amount) in coll_to_add.items():
            self.collateral_rewards_vault[asset] += amount
            self.cumulative_gains_total[asset] += amount

    # Test functions

    def user_usd_deposited(self, user):
        # return self.user_usd_deposits.get(user, 0)
        return self.get_compounded_usd_deposit(user)

    def usd_deposited(self):
        return self.total_usd_deposits

    def user_collateral_gained(self, user: str, asset: Coin):
        current_pending_gain = self.get_depositor_pending_gain(user, asset)
        collected_gain = self.cumulative_gains_per_user.get(
            user, default_coin_map(0)).get(asset, 0)
        return current_pending_gain + collected_gain

    def collateral_gained(self, asset: Coin):
        return self.cumulative_gains_total[asset]


def assert_equal(left, right):
    if left != right:
        print(f"{left} != {right}")
    assert left == right


def assert_equal_fuzzy(left, right, delta=0.01):
    if abs(left - right) > delta:
        print(f"{left} != {right}")
        assert left == right


def test_one():

    user_one = "one"
    user_two = "two"

    contract = SP()

    contract.approve_depositing(user_one)
    contract.approve_depositing(user_two)

    assert_equal(contract.user_usd_deposited(user_one), 0)
    assert_equal(contract.user_usd_deposited(user_two), 0)
    assert_equal(contract.usd_deposited(), 0)
    assert_equal(contract.user_collateral_gained(user_one, Coin.ETH), 0)
    assert_equal(contract.user_collateral_gained(user_two, Coin.ETH), 0)
    assert_equal(contract.collateral_gained(Coin.ETH), 0)

    contract.deposit(user_one, 100)
    assert_equal(contract.user_usd_deposited(user_one), 100)
    assert_equal(contract.user_usd_deposited(user_two), 0)
    assert_equal(contract.usd_deposited(), 100)
    assert_equal(contract.user_collateral_gained(user_one, Coin.ETH), 0)
    assert_equal(contract.user_collateral_gained(user_two, Coin.ETH), 0)
    assert_equal(contract.collateral_gained(Coin.ETH), 0)

    contract.deposit(user_two, 100)
    assert_equal(contract.user_usd_deposited(user_one), 100)
    assert_equal(contract.user_usd_deposited(user_two), 100)
    assert_equal(contract.usd_deposited(), 200)
    assert_equal(contract.user_collateral_gained(user_one, Coin.ETH), 0)
    assert_equal(contract.user_collateral_gained(user_two, Coin.ETH), 0)
    assert_equal(contract.collateral_gained(Coin.ETH), 0)

    # First event
    contract.liquidate(debt_to_offset=50, coll_to_add={Coin.ETH: 10})
    assert_equal(contract.user_usd_deposited(user_one), 75)
    assert_equal(contract.user_usd_deposited(user_two), 75)
    assert_equal(contract.usd_deposited(), 150)
    assert_equal(contract.user_collateral_gained(user_one, Coin.ETH), 5)
    assert_equal(contract.user_collateral_gained(user_two, Coin.ETH), 5)
    assert_equal(contract.collateral_gained(Coin.ETH), 10)

    contract.deposit(user_one, 100)
    assert_equal(contract.user_usd_deposited(user_one), 175)
    assert_equal(contract.user_usd_deposited(user_two), 75)
    assert_equal(contract.usd_deposited(), 250)

    # Second event
    contract.liquidate(debt_to_offset=100, coll_to_add={Coin.ETH: 150})

    assert_equal_fuzzy(contract.user_usd_deposited(user_one), 105)
    assert_equal_fuzzy(contract.user_usd_deposited(user_two), 45)
    assert_equal(contract.usd_deposited(), 150)
    assert_equal_fuzzy(
        contract.user_collateral_gained(user_one, Coin.ETH), 110)
    assert_equal_fuzzy(contract.user_collateral_gained(
        user_two, Coin.ETH), 5 + 45)
    assert_equal(contract.collateral_gained(Coin.ETH), 160)


def test_two_deposits_forces_harvest():
    user_one = "one"

    contract = SP()

    contract.approve_depositing(user_one)

    contract.deposit(user_one, 100)
    contract.liquidate(debt_to_offset=50, coll_to_add={
                       Coin.ETH: 150, Coin.SRM: 5})

    contract.deposit(user_one, 200)
    contract.liquidate(debt_to_offset=30, coll_to_add={
                       Coin.ETH: 150, Coin.BTC: 3})

    assert_equal_fuzzy(contract.user_usd_deposited(user_one), 220)
    assert_equal(contract.usd_deposited(), 220)

    assert_equal_fuzzy(
        contract.user_collateral_gained(user_one, Coin.ETH), 300)
    assert_equal_fuzzy(contract.user_collateral_gained(user_one, Coin.SRM), 5)
    assert_equal_fuzzy(contract.user_collateral_gained(user_one, Coin.BTC), 3)

    assert_equal(contract.collateral_gained(Coin.ETH), 300)
    assert_equal(contract.collateral_gained(Coin.SRM), 5)
    assert_equal(contract.collateral_gained(Coin.BTC), 3)
    assert_equal(contract.collateral_gained(Coin.SOL), 0)


def test_deposit_and_withdraw():
    user_one = "one"

    contract = SP()

    contract.approve_depositing(user_one)

    contract.deposit(user_one, 100)
    contract.liquidate(debt_to_offset=50, coll_to_add={
                       Coin.ETH: 150, Coin.SRM: 5})

    contract.deposit(user_one, 200)
    contract.liquidate(debt_to_offset=30, coll_to_add={
                       Coin.ETH: 150, Coin.BTC: 3})

    assert_equal_fuzzy(contract.user_usd_deposited(user_one), 220)
    assert_equal(contract.usd_deposited(), 220)

    assert_equal_fuzzy(
        contract.user_collateral_gained(user_one, Coin.ETH), 300)
    assert_equal_fuzzy(contract.user_collateral_gained(user_one, Coin.SRM), 5)
    assert_equal_fuzzy(contract.user_collateral_gained(user_one, Coin.BTC), 3)

    assert_equal(contract.collateral_gained(Coin.ETH), 300)
    assert_equal(contract.collateral_gained(Coin.SRM), 5)
    assert_equal(contract.collateral_gained(Coin.BTC), 3)
    assert_equal(contract.collateral_gained(Coin.SOL), 0)


def test_two_collaterals():
    user_one = "one"
    user_two = "two"

    contract = SP()

    contract.approve_depositing(user_one)
    contract.approve_depositing(user_two)

    contract.deposit(user_one, 150)
    contract.deposit(user_two, 300)

    contract.liquidate(
        debt_to_offset=150,
        coll_to_add={
            Coin.ETH: 150,
            Coin.SOL: 3
        }
    )

    assert_equal_fuzzy(contract.user_usd_deposited(user_one), 100)
    assert_equal_fuzzy(contract.user_usd_deposited(user_two), 200)

    assert_equal(contract.usd_deposited(), 300)

    assert_equal_fuzzy(
        contract.user_collateral_gained(user_one, Coin.ETH), 50)
    assert_equal_fuzzy(
        contract.user_collateral_gained(user_one, Coin.SOL), 1)

    assert_equal_fuzzy(contract.user_collateral_gained(
        user_two, Coin.ETH), 100)
    assert_equal_fuzzy(contract.user_collateral_gained(
        user_two, Coin.SOL), 2)

    assert_equal(contract.collateral_gained(Coin.ETH), 150)
    assert_equal(contract.collateral_gained(Coin.SOL), 3)


def test_simple_again():
    contract = SP()

    user_one = "one"
    user_two = "two"

    contract.approve_depositing(user_one)
    contract.approve_depositing(user_two)

    assert_equal(contract.user_usd_deposited(user_one), 0)
    assert_equal(contract.user_usd_deposited(user_two), 0)
    assert_equal(contract.usd_deposited(), 0)
    assert_equal(contract.user_collateral_gained(user_one, Coin.ETH), 0)
    assert_equal(contract.user_collateral_gained(user_two, Coin.ETH), 0)
    assert_equal(contract.collateral_gained(Coin.ETH), 0)

    contract.deposit(user_one, 100)
    assert_equal(contract.user_usd_deposited(user_one), 100)
    assert_equal(contract.user_usd_deposited(user_two), 0)
    assert_equal(contract.usd_deposited(), 100)
    assert_equal(contract.user_collateral_gained(user_one, Coin.ETH), 0)
    assert_equal(contract.user_collateral_gained(user_two, Coin.ETH), 0)
    assert_equal(contract.collateral_gained(Coin.ETH), 0)

    contract.deposit(user_two, 100)
    assert_equal(contract.user_usd_deposited(user_one), 100)
    assert_equal(contract.user_usd_deposited(user_two), 100)
    assert_equal(contract.usd_deposited(), 200)
    assert_equal(contract.user_collateral_gained(user_one, Coin.ETH), 0)
    assert_equal(contract.user_collateral_gained(user_two, Coin.ETH), 0)
    assert_equal(contract.collateral_gained(Coin.ETH), 0)

    # First event
    contract.liquidate(debt_to_offset=50, coll_to_add={Coin.ETH: 10})


if __name__ == "__main__":
    test_one()
    test_two_deposits_forces_harvest()
    test_two_collaterals()
    test_simple_again()
