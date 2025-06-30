//! Math for preserving precision of token amounts which are limited
//! by the SPL Token program to be at most u64::MAX.
//!
//! Decimals are internally scaled by a WAD (10^18) to preserve
//! precision up to 18 decimal places. Decimals are sized to support
//! both serialization and precise math for the full range of
//! unsigned 64-bit integers. The underlying representation is a
//! u192 rather than u320 to reduce compute cost while losing
//! support for arithmetic operations at the high end of u64 range.

#![allow(missing_docs, clippy::missing_docs_in_private_items)]

// use spl_math::{precise_number, uint::U256};
use std::{convert::TryFrom, fmt};

use crate::math::u256::U256;

mod consts {
    /// Scale of precision.
    pub(super) const SCALE: usize = 18;

    /// Identity
    pub(super) const WAD: u64 = 1_000_000_000_000_000_000;

    pub(super) const HALF_WAD: u64 = WAD / 2;
}

/// Large decimal values, precise to 18 digits
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct Decimal(pub U256);

impl Decimal {
    // OPTIMIZE: use const slice when fixed in BPF toolchain
    pub fn wad() -> U256 {
        U256::from(consts::WAD)
    }

    // OPTIMIZE: use const slice when fixed in BPF toolchain
    fn half_wad() -> U256 {
        U256::from(consts::HALF_WAD)
    }

    fn from_scaled_val(scaled_val: u128) -> Self {
        Self(U256::from(scaled_val))
    }

    pub fn checked_add(self, rhs: &Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(self, rhs: &Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    pub fn checked_div(self, rhs: &Self) -> Option<Self> {
        // Both the numerator `self.0` and the denominator `rhs.0` are scaled up
        // to 1E+18. Since we divide the numerator by the denominator we will
        // have to rescale the value. Depending on when we rescale the
        // calculation we run into lower risk of overflowing.
        match self.0.checked_mul(Self::wad()) {
            // We first try scale the numerator a second time to offset the
            // downscalling that occurs with the `checked_div`
            Some(v) => Some(Self(v.checked_div(rhs.0)?)),
            // If the numerator self.0 is bigger than 1E+39 = 1E+21 * 1E+18,
            // then it will overflow when multiplied by 1E+18 and therefore
            // `check_mul` will return None
            None => {
                let u192 = if self.0 >= rhs.0 {
                    // We divide numerator by denominator and
                    // scale up the result
                    self.0
                        .checked_div(rhs.0)
                        .and_then(|v| v.checked_mul(Self::wad()))
                } else {
                    // We downscale the denominator and then divide the
                    // scaled numerator by the unscaled denominator given
                    // the result is then the scaled result as desired
                    self.0.checked_div(rhs.0.checked_div(Self::wad())?)
                };

                u192.map(Self)
            }
        }
    }

    pub fn checked_mul(self, rhs: &Self) -> Option<Self> {
        match self.0.checked_mul(rhs.0) {
            Some(v) => Some(Self(v.checked_div(Self::wad())?)),
            None => {
                let u192 = if self.0 >= rhs.0 {
                    self.0
                        .checked_div(Self::wad())
                        .and_then(|v| v.checked_mul(rhs.0))
                } else {
                    rhs.0
                        .checked_div(Self::wad())
                        .and_then(|v| v.checked_mul(self.0))
                };

                u192.map(Self)
            }
        }
    }

    /// Approximate the square root using Newton's method.
    ///
    /// Based on <https://docs.rs/spl-math/0.1.0/spl_math/precise_number/struct.PreciseNumber.html#method.sqrt>
    pub fn checked_sqrt(&self) -> Option<Self> {
        let two = Self::from(2u64);
        let one = Self::from(1u64);
        // A good initial guess is the average of the interval that contains the
        // input number.  For all numbers, that will be between 1 and the given
        // number.
        let guess = self.clone().checked_add(&one)?.checked_div(&two)?;
        newtonian_root_approximation(self.clone(), two, guess)
    }

    pub fn checked_floor<T>(&self) -> Option<T>
    where
        T: TryFrom<U256>,
    {
        let ceil_val = self.0.checked_div(Self::wad())?;
        T::try_from(ceil_val).ok()
    }

    pub fn checked_ceil<T>(&self) -> Option<T>
    where
        T: TryFrom<U256>,
    {
        let ceil_val = Self::wad()
            .checked_sub(U256::from(1u64))?
            .checked_add(self.0)?
            .checked_div(Self::wad())?;
        T::try_from(ceil_val).ok()
    }

    fn checked_pow(&self, mut exp: u64) -> Option<Self> {
        let mut base = self.clone();
        let mut ret = if exp % 2 != 0 {
            base.clone()
        } else {
            Self::from(1u64)
        };

        loop {
            exp /= 2;
            if exp == 0 {
                break;
            }

            base = base.clone().checked_mul(&base)?;

            if exp % 2 != 0 {
                ret = ret.checked_mul(&base)?;
            }
        }

        Some(ret)
    }

    fn checked_round(&self) -> Option<u64> {
        let rounded_val = Self::half_wad()
            .checked_add(self.0)?
            .checked_div(Self::wad())?;
        u64::try_from(rounded_val).ok()
    }

    /// If the difference between self and other is less than 10^(dec_places -
    /// precision), return true.
    ///
    /// # Example
    /// If we have 18 decimal places, than having precision of 6 would mean that
    /// any difference beyond 12th dec place is considered as equal.
    pub fn almost_eq(&self, other: &Self, precision: u32) -> bool {
        let precision = Self::from_scaled_val(10u128.pow(precision));
        match self.cmp(other) {
            std::cmp::Ordering::Equal => true,
            std::cmp::Ordering::Less => other.clone().checked_sub(self).unwrap() < precision,
            std::cmp::Ordering::Greater => self.clone().checked_sub(other).unwrap() < precision,
        }
    }
}

impl From<u64> for Decimal {
    fn from(val: u64) -> Self {
        Self(Self::wad() * U256::from(val))
    }
}

impl From<u128> for Decimal {
    fn from(val: u128) -> Self {
        Self(Self::wad() * U256::from(val))
    }
}

impl From<&str> for Decimal {
    /// Converts a decimal string to U60x18 by scaling it up by 1e18.
    fn from(value: &str) -> Self {
        // Split the value into integer and fractional parts
        let parts: Vec<&str> = value.split('.').collect();

        let integer_part = parts[0];
        let fractional_part = if parts.len() > 1 { parts[1] } else { "0" };

        // Parse integer part
        let integer_value = U256::from_dec_str(integer_part).unwrap();
        let mut result = integer_value * consts::WAD;

        // Parse fractional part and scale it appropriately
        let mut fractional_value = U256::from(0);
        let scale_factor = 10u64.pow(fractional_part.len() as u32);

        if let Ok(parsed_fractional_value) = U256::from_dec_str(fractional_part) {
            fractional_value = (parsed_fractional_value * consts::WAD) / U256::from(scale_factor);
        }

        // Combine integer and fractional parts
        result += fractional_value;

        Decimal(result)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut scaled_val = self.0.to_string();
        if scaled_val.len() <= consts::SCALE {
            scaled_val.insert_str(0, &vec!["0"; consts::SCALE - scaled_val.len()].join(""));
            scaled_val.insert_str(0, "0.");
        } else {
            scaled_val.insert(scaled_val.len() - consts::SCALE, '.');
        }
        f.write_str(&scaled_val)
    }
}

/// Approximate the nth root of a number using Newton's method
/// <https://en.wikipedia.org/wiki/Newton%27s_method>
/// NOTE: this function is private because its accurate range and precision
/// have not been established.
///
/// Based on <https://docs.rs/spl-math/0.1.0/spl_math/precise_number/struct.PreciseNumber.html#method.sqrt>
fn newtonian_root_approximation(
    base: Decimal,
    root: Decimal,
    mut guess: Decimal,
) -> Option<Decimal> {
    const MAX_APPROXIMATION_ITERATIONS: u128 = 100;

    let zero = Decimal::from(0u64);
    if base == zero {
        return Some(zero);
    }
    if root == zero {
        return None;
    }
    let one = Decimal::from(1u64);
    let root_minus_one = root.clone().checked_sub(&one)?;
    let root_minus_one_whole = root_minus_one.checked_round()?;
    let mut last_guess = guess.clone();
    for _ in 0..MAX_APPROXIMATION_ITERATIONS {
        // x_k+1 = ((n - 1) * x_k + A / (x_k ^ (n - 1))) / n
        let first_term = root_minus_one.clone().checked_mul(&guess)?;
        let power = guess.clone().checked_pow(root_minus_one_whole);
        let second_term = match power {
            Some(num) => base.clone().checked_div(&num)?,
            None => Decimal::from(0u64),
        };
        guess = first_term.checked_add(&second_term)?.checked_div(&root)?;
        // the source uses precision of 2 places, but we originally used 3
        // places and want to keep the same precision as we tested our
        // programs with
        if last_guess.almost_eq(&guess, 3) {
            break;
        } else {
            last_guess = guess.clone();
        }
    }

    Some(guess)
}
