use crate::math::decimal::Decimal;

pub mod math;
pub mod omm_v2_0;
pub mod omm_v2_1;

/// Converts a btoken amount to its underlying amount using the btoken ratio.
pub fn to_underlying(btoken_amount: u64, b_token_ratio: &Decimal) -> u64 {
    (Decimal::from(btoken_amount)
        .checked_mul(b_token_ratio)
        .unwrap())
    .checked_floor::<u64>()
    .unwrap()
}

/// Converts an underlying amount to its btoken amount using the btoken ratio.
pub fn to_b_token(amount: u64, b_token_ratio: &Decimal) -> u64 {
    (Decimal::from(amount).checked_div(b_token_ratio).unwrap())
        .checked_floor::<u64>()
        .unwrap()
}
