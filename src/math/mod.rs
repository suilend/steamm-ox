// use crate::math::decimal::{self, Decimal, WAD};
// use crate::math::fixed_point::{self as fp64, FixedPoint64, SCALE_64};
// use crate::math::u256::{MAX_U128, MAX_U256, U256};

use crate::math::{
    decimal::Decimal,
    fixed_point::{FixedPoint64, FixedPointError},
    u256::U256,
};

pub mod decimal;
pub mod fixed_point;
pub mod u256;

const SCALE_64: u128 = 18446744073709551616;
const MAX_U128: u128 = 340282366920938463463374607431768211455;

pub fn decimal_to_fixedpoint64(d: Decimal) -> Result<FixedPoint64, FixedPointError> {
    let decimal_value = d.0;

    // It's safe to upscale the decimal value, given that
    // the maximum value inside a decimal type is MAX_U64 * WAD which is
    // roughly ≈ 1.844 × 10^37
    //
    // Multiplying it by 2^64 (SCALE_64) gives us a value of 3.4 × 10^56 which
    // is smaller than MAX_U256 (1.1579 × 10^77)
    let scaled_value = decimal_value * U256::from(SCALE_64) / Decimal::wad();
    if scaled_value > MAX_U128.into() {
        return Err(FixedPointError::OutOfRange(
            "Failed to convert decimal to fixed point: value too large".to_string(),
        ));
    }
    FixedPoint64::from_raw_value(scaled_value.as_u128())
}
