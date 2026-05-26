# Builders 2 - Week 4 - Assignment 1

# AMM - Automated Market Maker

A market making mechanism, that replaces tradtional buyers and sellers, automates the process where traders swap tokens directly from the pool, while algorithms automatically adjust prices based on a set of mathematical formulas

Problem it solves:

- Traditional exchanges need buyers AND sellers simultaneously
- If you want to buy 100 USDC, you need someone selling 100 USDC
  at that moment
- This is inefficient and slow

AMM solution:

- Create a pool with both tokens (USDC + SOL)
- Anyone can buy/sell from the pool anytime
- Pool automatically adjusts price based on demand (no order
  books)

---

# AMM math

## Constant Product Formula

### x \times y = k

- x = Token X balance in pool
- y = Token Y balance in pool
- k = x \* y (constant)

This constant NEVER changes during trades.

### Example

Initial pool:

- 1M USDC (x)
- 500K SOL (y)

### k = 1{,}000{,}000 \times 500{,}000 = 500{,}000{,}000{,}000

If someone deposits 10K USDC:

- New_X = 1,000,000 + 10,000 = 1,010,000

To maintain k:

### 1{,}010{,}000 \times New_Y = 500{,}000{,}000{,}000

- New_Y = 495,049.75 (about)
- Output = 4,950.25 SOL

### Price Check

Before:

- 500K / 1M = 0.5 SOL/USDC

After:

- 495K / 1.01M ≈ 0.49 SOL/USDC

(price increased as supply of SOL decreased)

---

## LP token mint calculation

### amount_x = \frac{amount_lp \times vault_x}{total_supply}

### amount_y = \frac{amount_lp \times vault_y}{total_supply}

Liqudity providers request any token x or y and along with mentions the lp token amount they want, in return the above equations calculate how much token y or x they need to provide inorder to maintain the current price ratio.

---

# Logic Explanation

- Automated Market Making starts with depositing token pairs into pool.
- Any Liquidity providers can provide token pairs into pool.
- In return they earn 0.3% of every swap, a seperate LP token is minted to track Liqudity providers share in the pool.
- Now with enough token pairs in pool, other people can swap their token and receieve the other, and the prize is adjusted based on the constant product formula.

---

# Architecture

```text
programs/builder_a1/src/
├── lib.rs
├── state.rs
├── error.rs
└── instructions/
    ├── mod.rs
    ├── swap.rs
    ├── deposit.rs
    ├── withdraw.rs
```

---

# File Overview

## lib.rs

Main anchor program entry point that declares 4 AMM instructions:

- initialize
- deposit
- withdraw
- swap

---

## state.rs

Defines `Config` account struct.

Stores:

- seed: pool identifier
- authority: Admin can lock pool
- mint_x: Token X mint address
- mint_y: Token Y mint address
- fee: Swap fee in basis points
- locked: Can be frozen by authority
- config_bump: config bump for PDA derivation
- lp_bump: lp bump for PDA derivation

---

# Instructions

Contains instructions for AMM program.

---

## instructions/mod.rs

Acts as an entry point module, where all the instructions are re-exported.

---

## instructions/initialize.rs

### GOAL: Create a new liquidity Pool

- creates token definitions for x,y and LP token mint PDAs
- creates token x,y vault PDAs
- creates config PDA for auth, pool details, fee, locked status etc...
- different seeds for different pools. so same token x,y pairs can have different pools.

---

## instructions/deposit.rs

### GOAL: Add liquidity to the pool, receieve LP tokens

- user provides both tokens x and y
- pool mints LP tokens proprotional to user's contribution
- LP tokens represent user's ownership share
- Liquidity providers earn a portion of swap fees
- added slippage protection

---

## instructions/withdraw.rs

### GOAL: Remove liquidity from pool, receive back tokens x,y

- User sends LP tokens to burn and destroy
- Pool sends back proportional tokens
- added slippage protection

---

## instructions/swap.rs

### GOAL: Exchange one token for another at pool price

- User provides one token
- Pool calculates output using constant product formula
- Pool deducts 0.3% fee → goes to Liquidity providers
- User receives output token

---

# Transaction Flows

## Deposit Transaction Flow

```text
User triggers:
deposit(amount_lp=1000, max_x=100, max_y=50)
        │
        ▼
[Validation]
- Pool not locked?
- amount > 0?
        │
        ▼
[Calculation]
Calculate x=100K, y=50K needed
        │
        ▼
[Slippage]
100K ≤ 100?
50K ≤ 50?
        │
        ▼
[Transfer 1]
Transfer 100K USDC
from user_x → vault_x

Authority: USER (signs)
CPI to Token Program
        │
        ▼
[Transfer 2]
Transfer 50K SOL
from user_y → vault_y

Authority: USER (signs)
CPI to Token Program
        │
        ▼
[Mint]
Mint 1000 LP tokens
to user_lp

Authority: CONFIG (PDA signs with seeds)
CPI to Token Program
        │
        ▼
Transaction Complete

Pool:
+100K USDC
+50K SOL
-1000 LP

User:
-100K USDC
-50K SOL
+1000 LP
```

---

## Swap Transaction Flow

```text
User triggers:
swap(is_x=true, amount=10000, min=4900)
        │
        ▼
[Validation]
- Pool not locked?
- amount > 0?
        │
        ▼
[Initialize Curve]
Set up x*y=k with fee
        │
        ▼
[Direction]
Selling X (buying Y)
→ LiquidityPair::X
        │
        ▼
[Calculate]

Apply fee:
10000 * 0.997 = 9970

Maintain k:
New_Y = k / (X + 9970)

Output:
4961 SOL
        │
        ▼
[Slippage]
4961 ≥ 4900?
        │
        ▼
[Transfer In]
Transfer 9970 USDC
from user_x → vault_x

Authority: USER (signs)
CPI to Token Program
        │
        ▼
[Transfer Out]
Transfer 4961 SOL
from vault_y → user_y

Authority: CONFIG (PDA signs with seeds)
CPI to Token Program
        │
        ▼
Transaction Complete

Pool:
+9970 USDC
-4961 SOL

User:
-10000 USDC (including fee)
+4961 SOL

LPs:
Automatically earn 30 USDC fee worth
(proportional ownership)
```

---

## Withdraw Transaction Flow

```text
User triggers:
withdraw(amount_lp=1000, min_x=95000, min_y=47500)
        │
        ▼
[Validation]
- Pool locked?
- amount > 0?
- user owns LP?
        │
        ▼
[Calculation]
Calculate x=100K, y=50K to withdraw
        │
        ▼
[Slippage]
100K ≥ 95K?
50K ≥ 47.5K?
        │
        ▼
[Transfer 1]
Transfer 100K USDC
from vault_x → user_x

Authority: CONFIG (PDA signs with seeds)
CPI to Token Program
        │
        ▼
[Transfer 2]
Transfer 50K SOL
from vault_y → user_y

Authority: CONFIG (PDA signs with seeds)
CPI to Token Program
        │
        ▼
[Burn]
Burn 1000 LP tokens
from user_lp

Authority: USER
(signs - burning own tokens)

CPI to Token Program
        │
        ▼
Transaction Complete

Pool:
-100K USDC
-50K SOL
-1000 LP

User:
+100K USDC
+50K SOL
-1000 LP
```

---

# Results

## Build Passed Succesfully

![alt text](../assets/)

---

## Tests Passed Succesfully

![alt text](../assets/)
