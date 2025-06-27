use crate::{
    math::{decimal::Decimal, fixed_point::FixedPointError, u256::U256},
    to_b_token, to_underlying,
};

const SCALE: u64 = 10000000000;
const A_PRECISION: u128 = 100;
const LIMIT: usize = 255;

// === Swap Functions ===

pub fn quote_swap(
    // Amount in (btoken token - e.g. bSUI or bUSDC)
    b_token_amount_in: u64,
    // Reserve X (btoken token - e.g. bSUI)
    b_token_reserve_x: u64,
    // Reserve Y (btoken token - e.g. bUSDC)
    b_token_reserve_y: u64,
    // Price X (underlying price - e.g. 3 SUI)
    price_x: Decimal,
    // Price Y (underlying price - e.g. 1 USDC)
    price_y: Decimal,
    decimals_x: u32,
    decimals_y: u32,
    amplifier: u32,
    x2y: bool,
    b_token_ratio_x: Decimal,
    b_token_ratio_y: Decimal,
) -> Result<u64, FixedPointError> {
    let amount_in = to_underlying(
        b_token_amount_in,
        if x2y {
            &b_token_ratio_x
        } else {
            &b_token_ratio_y
        },
    );

    let reserve_x = to_underlying(b_token_reserve_x, &b_token_ratio_x);
    let reserve_y = to_underlying(b_token_reserve_y, &b_token_ratio_y);

    let (price_x_integer_part, price_x_decimal_part_inverted) = split_price(price_x);
    let (price_y_integer_part, price_y_decimal_part_inverted) = split_price(price_y);

    // We avoid using Decimal and use u256 instead to increase the overflow limit
    // Reserves are in USD value and scaled by 10^10
    let scaled_usd_reserve_x = {
        let scaled_reserve = U256::from(reserve_x) * U256::from(SCALE);
        let scaled_reserve_usd = to_usd(
            scaled_reserve,
            price_x_integer_part,
            price_x_decimal_part_inverted,
        );
        scaled_reserve_usd / (10_u64.pow(decimals_x))
    };

    let scaled_usd_reserve_y = {
        let scaled_reserve = U256::from(reserve_y) * U256::from(SCALE);
        let scaled_reserve_usd = to_usd(
            scaled_reserve,
            price_y_integer_part,
            price_y_decimal_part_inverted,
        );

        scaled_reserve_usd / 10_u64.pow(decimals_y)
    };

    // We follow the Curve convention where the amplifier is actually defined as
    // A * n^(n-1) * A_PRECISION => A * 2^1 * A_PRECISION
    let scaled_amp = U256::from(amplifier * 2) * U256::from(A_PRECISION);
    let d = get_d(scaled_usd_reserve_x, scaled_usd_reserve_y, scaled_amp);

    let scaled_amount_in = U256::from(amount_in) * U256::from(SCALE);

    let amount_out_btoken = if x2y {
        let scaled_usd_amount_in = to_usd(
            scaled_amount_in,
            price_x_integer_part,
            price_x_decimal_part_inverted,
        ) / 10_u64.pow(decimals_x);

        let scaled_usd_reserve_out_after_trade =
            get_y(scaled_usd_reserve_x + scaled_usd_amount_in, scaled_amp, d);

        let scaled_reserve_out_after_trade = from_usd(
            scaled_usd_reserve_out_after_trade,
            price_y_integer_part,
            price_y_decimal_part_inverted,
        );

        let reserve_out_after_trade =
            (scaled_reserve_out_after_trade * 10_u64.pow(decimals_y) / U256::from(SCALE)).as_u64();

        let amount_out_underlying = reserve_y - reserve_out_after_trade;
        let amount_out_btoken = to_b_token(amount_out_underlying, &b_token_ratio_y);

        if amount_out_btoken > b_token_reserve_y {
            return Ok(0);
        }
        amount_out_btoken
    } else {
        let scaled_usd_amount_in = to_usd(
            scaled_amount_in,
            price_y_integer_part,
            price_y_decimal_part_inverted,
        ) / 10_u64.pow(decimals_y);

        let scaled_usd_reserve_out_after_trade =
            get_y(scaled_usd_reserve_y + scaled_usd_amount_in, scaled_amp, d);

        let scaled_reserve_out_after_trade = from_usd(
            scaled_usd_reserve_out_after_trade,
            price_x_integer_part,
            price_x_decimal_part_inverted,
        );

        let reserve_out_after_trade =
            (scaled_reserve_out_after_trade * 10_u64.pow(decimals_x) / U256::from(SCALE)).as_u64();

        let amount_out_underlying = reserve_x - reserve_out_after_trade;
        let amount_out_btoken = to_b_token(amount_out_underlying, &b_token_ratio_x);

        if amount_out_btoken > b_token_reserve_x {
            return Ok(0);
        }
        amount_out_btoken
    };

    Ok(amount_out_btoken)
}

/// Splits the price into integer and decimal part, using U256.
/// The decimal part is inverted and floored to U256, or None if no decimal part.
pub fn split_price(price: Decimal) -> (U256, Option<U256>) {
    let price_integer_part_u64 = price.checked_floor::<u64>().unwrap();
    let price_integer_part = U256::from(price_integer_part_u64);
    let price_decimal_part = price
        .checked_sub(&Decimal::from(price_integer_part_u64))
        .unwrap();

    if price_decimal_part.eq(&Decimal::from(0_u64)) {
        (price_integer_part, None)
    } else {
        let price_decimal_part_inverted = (Decimal::from(1_u64).checked_div(&price_decimal_part))
            .unwrap()
            .checked_floor::<u64>()
            .unwrap();
        (
            price_integer_part,
            Some(U256::from(price_decimal_part_inverted)),
        )
    }
}

/// Converts a unit amount into a USD amount using split price.
pub fn to_usd(
    amount: U256,
    price_integer_part: U256,
    price_decimal_part_inverted: Option<U256>,
) -> U256 {
    match price_decimal_part_inverted {
        Some(inv) => amount * price_integer_part + amount / inv,
        None => amount * price_integer_part,
    }
}

/// Converts a USD amount into a unit amount using split price.
pub fn from_usd(
    usd_amount: U256,
    price_integer_part: U256,
    price_decimal_part_inverted: Option<U256>,
) -> U256 {
    match price_decimal_part_inverted {
        Some(inv) => usd_amount * inv / (price_integer_part * inv + U256::one()),
        None => usd_amount / price_integer_part,
    }
}

/// Calculates the D invariant for a 2-coin pool using integer math.
/// Returns D as U256 or panics if it does not converge.
pub fn get_d(reserve_a: U256, reserve_b: U256, amp: U256) -> U256 {
    let sum = reserve_a + reserve_b;
    let ann = amp * U256::from(2u8); // n = 2 coins

    let mut d = sum;
    let mut limit = LIMIT;

    while limit > 0 {
        let mut d_p = d;
        d_p = d_p * d / reserve_a;
        d_p = d_p * d / reserve_b;
        d_p = d_p / U256::from(4u8);

        let d_prev = d;

        let numerator = ((ann * sum / U256::from(A_PRECISION)) + d_p * U256::from(2u8)) * d;
        let denominator = ((ann - U256::from(A_PRECISION)) * d / U256::from(A_PRECISION))
            + (U256::from(3u8) * d_p);

        d = numerator / denominator;

        if d > d_prev {
            if d - d_prev <= U256::one() {
                return d;
            }
        } else {
            if d_prev - d <= U256::one() {
                return d;
            }
        }

        limit -= 1;
    }

    panic!("get_d did not converge");
}

/// Calculates the output reserve after a swap using the StableSwap invariant.
/// Returns the new reserve as U256 or panics if it does not converge.
pub fn get_y(reserve_in: U256, amp: U256, d: U256) -> U256 {
    let ann = amp * U256::from(2u8);

    let sum = reserve_in;
    let mut c = d * d / (U256::from(2u8) * reserve_in);
    c = c * d * U256::from(A_PRECISION) / (ann * U256::from(2u8));

    let b = sum + d * U256::from(A_PRECISION) / ann;
    let mut y_prev;
    let mut y = d;

    let mut limit = LIMIT;

    while limit > 0 {
        y_prev = y;
        y = (y * y + c) / (U256::from(2u8) * y + b - d);

        if y > y_prev {
            if y - y_prev <= U256::one() {
                return y;
            }
        } else {
            if y_prev - y <= U256::one() {
                return y;
            }
        }

        limit -= 1;
    }

    panic!("get_y did not converge");
}
