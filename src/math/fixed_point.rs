use anyhow::Result;
use std::convert::TryInto;

use crate::math::u256::U256;
use std::fmt;

const LN2: u128 = 12_786_308_645_202_655_660; // ln(2) in fixed 64 representation
const MAX_U128: u128 = 340_282_366_920_938_463_463_374_607_431_768_211_455; // 2^128 - 1

// === Errors ===
// #[derive(Debug)]
// pub enum FixedPointError {
//     OutOfRange(String),
//     ZeroDivision,
//     NegativeResult,
//     Overflow(String),
//     LogOfZero,
//     SqrtOfNegative,
//     AssertionFailed(String),
// }

// impl std::fmt::Display for FixedPointError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             FixedPointError::OutOfRange(msg) => write!(f, "Value out of range: {}", msg),
//             FixedPointError::ZeroDivision => write!(f, "Zero division"),
//             FixedPointError::NegativeResult => write!(f, "Negative result"),
//             FixedPointError::Overflow(msg) => write!(f, "Overflow: {}", msg),
//             FixedPointError::LogOfZero => write!(f, "Log of zero"),
//             FixedPointError::SqrtOfNegative => write!(f, "Square root of negative number"),
//             FixedPointError::AssertionFailed(msg) => write!(f, "Assertion failed: {}", msg),
//         }
//     }
// }

// impl std::error::Error for FixedPointError {}

// === FixedPoint64 Struct ===
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FixedPoint64 {
    value: u128,
}

impl fmt::Display for FixedPoint64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Scaling factor is 2^64
        let scaling_factor = 1u128 << 64;
        let raw_value = self.value;

        let integer_part = raw_value / scaling_factor;
        let fractional_part = raw_value % scaling_factor;

        // Use 10^18 to get 18 decimal places
        let decimal_scaling = 1_000_000_000_000_000_000u128;
        let fractional_display =
            (fractional_part as u128).saturating_mul(decimal_scaling) / scaling_factor;

        // Pad fractional part with leading zeros to 18 digits
        write!(f, "{}.{:018}", integer_part, fractional_display)
    }
}

impl FixedPoint64 {
    pub fn new(value: u128) -> Result<Self> {
        if value > MAX_U128 {
            return Err(anyhow::anyhow!("Value out of range: {}", value));
        }
        Ok(FixedPoint64 { value })
    }

    // === Public-View Functions ===
    pub fn get_value(&self) -> u128 {
        self.value
    }

    // === Convert Functions ===
    pub fn from(value: u128) -> Result<Self> {
        let scaled_value = value
            .checked_shl(64)
            .ok_or_else(|| anyhow::anyhow!("Shift overflow"))?;
        Self::new(scaled_value)
    }

    pub fn one() -> Result<Self> {
        Self::from(1)
    }

    pub fn zero() -> Result<Self> {
        Self::from(0)
    }

    pub fn from_raw_value(value: u128) -> Result<Self> {
        Self::new(value)
    }

    pub fn from_rational(numerator: u128, denominator: u128) -> Result<Self> {
        if denominator == 0 {
            return Err(anyhow::anyhow!("Zero division"));
        }
        let scaled_numerator = numerator
            .checked_shl(64)
            .ok_or_else(|| anyhow::anyhow!("Shift overflow"))?;
        let quotient = scaled_numerator / denominator;
        if quotient == 0 && numerator != 0 {
            return Err(anyhow::anyhow!("Result too small"));
        }
        if quotient > MAX_U128 {
            return Err(anyhow::anyhow!("Result too large"));
        }
        Self::new(quotient)
    }

    pub fn to_u128(&self) -> u128 {
        let floored_num = self.to_u128_down() << 64;
        let boundary = floored_num + (1_u128 << 63);
        if self.value < boundary {
            self.to_u128_down()
        } else {
            self.to_u128_up()
        }
    }

    pub fn to_u128_down(&self) -> u128 {
        self.value >> 64
    }

    pub fn to_u128_up(&self) -> u128 {
        let floored_num = self.to_u128_down() << 64;
        if self.value == floored_num {
            floored_num >> 64
        } else {
            (floored_num + (1_u128 << 64)) >> 64
        }
    }

    // === Comparison Functions ===
    pub fn is_zero(&self) -> bool {
        self.value == 0
    }

    pub fn equals(&self, other: &Self) -> bool {
        self.value == other.value
    }

    pub fn lt(&self, other: &Self) -> bool {
        self.value < other.value
    }

    pub fn gt(&self, other: &Self) -> bool {
        self.value > other.value
    }

    pub fn lte(&self, other: &Self) -> bool {
        self.value <= other.value
    }

    pub fn gte(&self, other: &Self) -> bool {
        self.value >= other.value
    }

    pub fn max(x: Self, y: Self) -> Self {
        if x.value > y.value { x } else { y }
    }

    pub fn min(x: Self, y: Self) -> Self {
        if x.value < y.value { x } else { y }
    }

    // === Math Operations ===
    pub fn sub(&self, other: &Self) -> Result<Self> {
        if self.value < other.value {
            return Err(anyhow::anyhow!("Negative result"));
        }
        Self::new(self.value - other.value)
    }

    pub fn add(&self, other: &Self) -> Result<Self> {
        let result = self
            .value
            .checked_add(other.value)
            .ok_or_else(|| anyhow::anyhow!("Addition overflow"))?;
        Self::new(result)
    }

    pub fn mul(&self, other: &Self) -> Result<Self> {
        // Convert u128 values to U256 for multiplication
        let x = U256::from(self.value);
        let y = U256::from(other.value);

        // Perform multiplication and right-shift by 64
        let product = (x * y) >> 64;

        // Convert back to u128, checking for overflow
        let result: u128 = product
            .try_into()
            .map_err(|_| anyhow::anyhow!("U256 to u128 conversion overflow (mul)"))?;

        Self::new(result)
    }

    pub fn div(&self, other: &Self) -> Result<Self> {
        if other.value == 0 {
            return Err(anyhow::anyhow!("Zero division"));
        }

        // Convert u128 values to U256
        let x = U256::from(self.value);
        let y = U256::from(other.value);

        // Left-shift numerator by 64 bits
        let shifted_x = x << 64;

        // Perform division using math256::div_down (assumed to take U256)
        let result = shifted_x / y;

        // Convert back to u128, checking for overflow
        let result_u128: u128 = result
            .try_into()
            .map_err(|_| anyhow::anyhow!("U256 to u128 conversion overflow (div)"))?;

        Self::new(result_u128)
    }

    pub fn pow(&self, exponent: u32) -> Result<Self> {
        let raw_value = pow_raw(self.value.into(), exponent as u128)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("U256 to u128 conversion overflow (pow)"))?;

        Self::new(raw_value)
    }

    pub fn log2_plus_64(&self) -> Result<Self> {
        log2_64(self.value)
    }

    pub fn ln_plus_64ln2(&self) -> Result<Self> {
        // Compute log2_64 of self.value
        let x = log2_64(self.value)?.value;

        // Convert to U256 for multiplication
        let x_u256 = U256::from(x);
        let ln2_u256 = U256::from(LN2);

        // Perform multiplication and right-shift by 64
        let result = (x_u256 * ln2_u256) >> 64;

        // Convert back to u128, checking for overflow
        let result_u128: u128 = result
            .try_into()
            .map_err(|_| anyhow::anyhow!("U256 to u128 conversion overflow (pow)"))?;

        Self::from_raw_value(result_u128)
    }

    /// Computes (n1 * n2 * ... * nk) / (d1 * d2 * ... * dm) with checks for overflow, zero division, and precision loss.
    /// The computation schedules multiplications and divisions to maximize precision and minimize overflow risk.
    /// Numerators and denominators are sorted in descending order before processing.
    pub fn multiply_divide(
        numerators: &mut Vec<FixedPoint64>,
        denominators: &mut Vec<FixedPoint64>,
    ) -> Result<FixedPoint64> {
        if numerators.is_empty() {
            return Err(anyhow::anyhow!("No numerators"));
        }

        // Sort numerators and denominators in descending order
        sort_descending(numerators);
        sort_descending(denominators);

        // Initialize result to 1.0 (2^64 in FixedPoint64)
        let mut result = FixedPoint64::one()?;

        let mut num_idx = numerators.len();
        let mut den_idx = denominators.len();

        // Process numerators
        while num_idx > 0 {
            let numerator = numerators[num_idx - 1];
            match result.mul(&numerator) {
                Ok(product) => {
                    result = product;
                    num_idx -= 1;
                }
                Err(_) => {
                    // Multiplication failed (overflow), try to divide
                    if den_idx == 0 {
                        return Err(anyhow::anyhow!("Multiplication overflow"));
                    }
                    let denominator = denominators[den_idx - 1];
                    result = result.div(&denominator)?;
                    den_idx -= 1;
                }
            }
        }

        // Process remaining denominators
        while den_idx > 0 {
            let denominator = denominators[den_idx - 1];
            result = result.div(&denominator)?;
            den_idx -= 1;
        }

        Ok(result)
    }
}

// === Private Helper Functions ===

/// Sorts a mutable slice of FixedPoint64 in descending order using insertion sort.
/// Efficient for small slices (length <= 3).
fn sort_descending(v: &mut [FixedPoint64]) {
    let len = v.len();
    if len <= 1 {
        return;
    }
    for i in 1..len {
        let mut j = i;
        while j > 0 && v[j - 1].value < v[j].value {
            v.swap(j - 1, j);
            j -= 1;
        }
    }
}

pub(crate) fn pow_raw(x: U256, n: u128) -> Result<U256> {
    let mut res = U256::from(1_u128) << 64;
    let mut n_mut = n;
    let mut x_mut = x;
    while n_mut != 0 {
        if n_mut & 1 != 0 {
            res = res
                .checked_mul(x_mut)
                .ok_or_else(|| anyhow::anyhow!("Multiplication overflow (pow_raw_1)"))?
                >> 64;
        }
        n_mut >>= 1;
        x_mut = x_mut
            .checked_mul(x_mut)
            .ok_or_else(|| anyhow::anyhow!("Multiplication overflow (pow_raw)"))?
            >> 64;
    }
    Ok(res)
}

pub(crate) fn floor_log2(x: u128) -> Result<u32> {
    if x == 0 {
        return Err(anyhow::anyhow!("Log of zero"));
    }
    let mut res = 0;
    let mut x_mut = x;
    let mut n = 64;
    while n > 0 {
        if x_mut >= 1_u128 << n {
            x_mut >>= n;
            res += n;
        }
        n >>= 1;
    }
    Ok(res)
}

pub(crate) fn log2_64(x: u128) -> Result<FixedPoint64> {
    let mut x_mut = x;
    let integer_part = floor_log2(x_mut)?;
    if x_mut >= 1_u128 << 63 {
        x_mut >>= integer_part - 63;
    } else {
        x_mut <<= 63 - integer_part;
    }
    let mut frac = 0_u128;
    let mut delta = 1_u128 << 63;
    while delta != 0 {
        x_mut = (x_mut)
            .checked_mul(x_mut)
            .ok_or_else(|| anyhow::anyhow!("Multiplication overflow"))?
            >> 63;
        if x_mut >= 2_u128 << 63 {
            frac += delta;
            x_mut >>= 1;
        }
        delta >>= 1;
    }
    let result = (integer_part as u128)
        .checked_shl(64)
        .ok_or_else(|| anyhow::anyhow!("Shift overflow"))?
        .checked_add(frac)
        .ok_or_else(|| anyhow::anyhow!("Addition overflow"))?;
    FixedPoint64::from_raw_value(result)
}
