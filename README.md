# Steamm-Ox
Offchain Logic for Computing Quotes in STEAMM

## Overview
Steamm-Ox provides the offchain logic for calculating quotes in STEAMM's Oracle (OMM) pools. This documentation details the migration plan for updating the pricing system in these pools.

## OMM Migration Plan

The STEAMM OMM pools are transitioning to a new pricing system based on Curve's StableSwap formula. Both the legacy and new systems use USD values derived from token balances, leveraging Pyth oracle prices as the starting point. The key difference lies in the pricing formula: the new system adopts Curve's StableSwap approach for improved rebalancing mechanism.

### Key Changes
- **Interface Continuity**: The interface and function signatures remain unchanged, ensuring no modifications to the transaction workflow are required.
- **Internal Logic Update**: The pricing formula is updated to use Curve's StableSwap logic, affecting how prices are simulated.
- **Migration Trigger**: The new logic activates when a swap occurs, and the pool ratio (in USD) is within the 40:60 to 60:40 range. Outside this range, the legacy logic applies until a qualifying swap triggers the migration.
- **Amplifier Mapping**: Amplifiers will be remapped to align with the new StableSwap formula. Details of this mapping are still being finalized and will be provided soon.
- **Spread Fee Adjustment**: A spread fee is introduced based on Pyth's price confidence interval. If `price_uncertainty / price` exceeds `swap_fee_bps`, the higher of the two is applied.

## Offchain Logic

Below are examples of the old and new logic for both Rust and TypeScript implementations.

### Rust Implementation

#### Legacy Logic
```rust
let quote = omm::omm_v2_legacy::quote_swap(
    10_000_000,             // Amount in (btoken)
    1_000_000_000_000,      // Reserve X (btoken)
    1_000_000_000,          // Reserve Y (btoken)
    Decimal::from("3"),     // Price of X (from Pyth)
    Decimal::from("1"),     // Price of Y (from Pyth)
    9,                      // Decimals X
    6,                      // Decimals Y
    30,                     // Amplifier
    false,                  // x2y (swap direction)
    Decimal::from("1.0"),   // BTokenRatio X
    Decimal::from("1.0"),   // BTokenRatio Y
    50,                     // Swap Fees (BPS)
)?;
```

#### New Logic
```rust
let quote = omm::omm_v2_new::quote_swap(
    10_000_000,             // Amount in (btoken)
    1_000_000_000_000,      // Reserve X (btoken)
    1_000_000_000,          // Reserve Y (btoken)
    Decimal::from("3"),     // Price of X (from Pyth)
    Decimal::from("1"),     // Price of Y (from Pyth)
    9,                      // Decimals X
    6,                      // Decimals Y
    30,                     // Amplifier
    false,                  // x2y (swap direction)
    Decimal::from("1.0"),   // BTokenRatio X
    Decimal::from("1.0"),   // BTokenRatio Y
    50,                     // Swap Fees (BPS)
    Decimal::from("0.02"),  // Price Confidence X
    Decimal::from("0.001"), // Price Confidence Y
)?;
```

### TypeScript Implementation

#### Legacy Logic
```typescript
const quote = ommV2Legacy.quoteSwap(
    new BN(10_000_000),         // Amount in (btoken)
    new BN(1_000_000_000_000),  // Reserve X (btoken)
    new BN(1_000_000_000),      // Reserve Y (btoken)
    3,                          // Price of X (from Pyth)
    1,                          // Price of Y (from Pyth)
    9,                          // Decimals X
    6,                          // Decimals Y
    1,                          // Amplifier
    false,                      // x2y (swap direction)
    new Decimal("1.0"),         // BTokenRatio X
    new Decimal("1.0"),         // BTokenRatio Y
    BigInt(50),                 // Swap Fees (BPS)
);
```

#### New Logic
```typescript
const quote = ommV2New.quoteSwap(
    new BN(10_000_000),         // Amount in (btoken)
    new BN(1_000_000_000_000),  // Reserve X (btoken)
    new BN(1_000_000_000),      // Reserve Y (btoken)
    3,                          // Price of X (from Pyth)
    1,                          // Price of Y (from Pyth)
    9,                          // Decimals X
    6,                          // Decimals Y
    1,                          // Amplifier
    false,                      // x2y (swap direction)
    new Decimal("1.0"),         // BTokenRatio X
    new Decimal("1.0"),         // BTokenRatio Y
    BigInt(50),                 // Swap Fees (BPS)
    new Decimal("0.02"),        // Price Confidence X
    new Decimal("0.001"),       // Price Confidence Y
);
```

## Notes
- The new logic introduces `Price Confidence` parameters for both tokens to account for Pyth's price uncertainty.
- Monitor pool ratios (in usd values) to anticipate when the migration will occur.
- Amplifier mapping details will be shared once finalized.