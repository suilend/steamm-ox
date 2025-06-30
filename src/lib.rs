use crate::math::{decimal::Decimal, safe_mul_div_up};
use anyhow::Result;

pub mod math;
pub mod omm;

pub const BPS_SCALE: u64 = 10_000; // Basis points scale factor
const PROTOCOL_FEE_NUMERATOR: u64 = 2_000;

pub struct SwapQuote {
    pub amount_in: u64,
    pub amount_out: u64,
    pub protocol_fees: u64,
    pub pool_fees: u64,
    pub a2b: bool,
}

pub fn compute_swap_fees(
    amount: u64,
    swap_fee_bps: u64,
    swap_fee_override_numerator: Option<u64>,
) -> Result<(u64, u64)> {
    let (protocol_fee_num, protocol_fee_denom) = (PROTOCOL_FEE_NUMERATOR, BPS_SCALE);
    let (pool_fee_num, pool_fee_denom) = if let Some(override_num) = swap_fee_override_numerator {
        let (pool_fee_num_default, pool_fee_denom_default) = (swap_fee_bps, BPS_SCALE);
        if override_num * pool_fee_denom_default > pool_fee_num_default * BPS_SCALE {
            (override_num, BPS_SCALE)
        } else {
            (pool_fee_num_default, pool_fee_denom_default)
        }
    } else {
        (swap_fee_bps, BPS_SCALE)
    };

    let total_fees = safe_mul_div_up(amount, pool_fee_num, pool_fee_denom)?;
    let protocol_fees = safe_mul_div_up(total_fees, protocol_fee_num, protocol_fee_denom)?;
    let pool_fees = total_fees - protocol_fees;

    Ok((protocol_fees, pool_fees))
}

pub fn get_quote(
    amount_in: u64,
    amount_out: u64,
    a2b: bool,
    swap_fee_bps: u64,
    swap_fee_override_numerator: Option<u64>,
) -> SwapQuote {
    let (protocol_fees, pool_fees) =
        compute_swap_fees(amount_out, swap_fee_bps, swap_fee_override_numerator).unwrap();
    let amount_out_net = amount_out
        .saturating_sub(protocol_fees)
        .saturating_sub(pool_fees);

    SwapQuote {
        amount_in,
        amount_out: amount_out_net,
        protocol_fees,
        pool_fees,
        a2b,
    }
}

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
