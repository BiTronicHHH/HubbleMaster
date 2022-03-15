from random import random, randint

coins = ["BTC", "USD", "ETH", "SOL", "SRM"]
users = []
for i in range(50):
    assets = []
    for j in range(5):
        pos = round(random() * 10000, 2)
        asset = randint(0, len(coins) - 1)
        assets.append(f"({pos}, {asset})")
    users.append(f"[{','.join(assets)}]")

print (",".join(users))
