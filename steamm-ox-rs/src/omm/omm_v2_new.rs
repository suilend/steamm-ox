use crate::{
    BPS_SCALE, SwapQuote, get_quote,
    math::{decimal::Decimal, u256::U256},
    to_b_token, to_underlying,
};
use anyhow::Result;

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
    swap_fee_bps: u64,
    price_confidence_a: Decimal,
    price_confidence_b: Decimal,
) -> Result<SwapQuote> {
    let amount_out_btoken = quote_swap_no_fees(
        b_token_amount_in,
        b_token_reserve_x,
        b_token_reserve_y,
        price_x.clone(),
        price_y.clone(),
        decimals_x,
        decimals_y,
        amplifier,
        x2y,
        b_token_ratio_x,
        b_token_ratio_y,
    )?;

    let price_uncertainty_ratio_a = price_uncertainty_ratio(price_x, price_confidence_a)?;
    let price_uncertainty_ratio_b = price_uncertainty_ratio(price_y, price_confidence_b)?;

    Ok(get_quote(
        b_token_amount_in,
        amount_out_btoken,
        x2y,
        swap_fee_bps,
        Some(price_uncertainty_ratio_a.max(price_uncertainty_ratio_b)),
    ))
}

fn price_uncertainty_ratio(price: Decimal, price_confidence: Decimal) -> Result<u64> {
    Ok(price_confidence
        .checked_mul(&Decimal::from(BPS_SCALE))
        .ok_or_else(|| anyhow::anyhow!("Multiplication failed"))?
        .checked_div(&price)
        .ok_or_else(|| anyhow::anyhow!("Division failed"))?
        .checked_floor()
        .ok_or_else(|| anyhow::anyhow!("Floor failed"))?)
}

pub fn quote_swap_no_fees(
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
) -> Result<u64> {
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

    let (price_x_integer_part, price_x_decimal_part_inverted) = split_price(price_x)?;
    let (price_y_integer_part, price_y_decimal_part_inverted) = split_price(price_y)?;

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

        let amount_out_underlying = reserve_y - reserve_out_after_trade - 1;
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

        let amount_out_underlying = reserve_x - reserve_out_after_trade - 1;
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
pub fn split_price(price: Decimal) -> Result<(U256, Option<U256>)> {
    let price_integer_part_u64 = price
        .checked_floor::<u64>()
        .ok_or_else(|| anyhow::anyhow!("Failed to get integer part of price"))?;

    let price_integer_part = U256::from(price_integer_part_u64);
    let price_decimal_part = price
        .checked_sub(&Decimal::from(price_integer_part_u64))
        .ok_or_else(|| anyhow::anyhow!("Failed to get decimal part of price"))?;

    if price_decimal_part.eq(&Decimal::from(0_u64)) {
        Ok((price_integer_part, None))
    } else {
        let price_decimal_part_inverted = (Decimal::from(1_u64).checked_div(&price_decimal_part))
            .ok_or_else(|| anyhow::anyhow!("Failed to invert decimal part of price"))?
            .checked_floor::<u64>()
            .ok_or_else(|| anyhow::anyhow!("Failed to floor inverted decimal part of price"))?;
        Ok((
            price_integer_part,
            Some(U256::from(price_decimal_part_inverted)),
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_swap() -> Result<()> {
        // // Test case 1
        let amt_out = quote_swap_no_fees(
            10_000_000,        // 10 * 10^6
            1_000_000_000_000, // 1_000 * 10^9
            1_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            false,
            Decimal::from("1.0"),
            Decimal::from("1.0"),
        )?;
        assert_eq!(amt_out, 5_156_539_130, "Test case 1 failed");

        // Test case 2
        let amt_out = quote_swap_no_fees(
            100_000_000,       // 100 * 10^6
            1_000_000_000_000, // 1_000 * 10^9
            1_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            false,
            Decimal::from("1.0"),
            Decimal::from("1.0"),
        )?;
        assert_eq!(amt_out, 49_852_725_213, "Test case 2 failed");

        // Test case 3
        let amt_out = quote_swap_no_fees(
            5_156_539_131,     // 5.15 SUI
            1_000_000_000_000, // 1_000 * 10^9
            1_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            true,
            Decimal::from("1.0"),
            Decimal::from("1.0"),
        )?;
        assert_eq!(amt_out, 9_920_471, "Test case 3 failed");

        Ok(())
    }

    #[test]
    fn test_quote_swap_with_different_btoken_ratios() -> Result<()> {
        // Test case 1
        let amt_out = quote_swap_no_fees(
            11_000_000,        // 10 * 10^6 * 1.1
            1_000_000_000_000, // 1_000 * 10^9
            3_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            false,
            Decimal::from("1.0"),
            Decimal::from("1.0")
                .checked_div(&Decimal::from("1.1"))
                .unwrap(),
        )?;
        assert_eq!(amt_out, 3_437_018_128, "Test case 1 failed");

        let amt_out = quote_swap_no_fees(
            10_000_000,        // 10 * 10^6
            1_000_000_000_000, // 1_000 * 10^9
            3_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            false,
            Decimal::from("0.5"),
            Decimal::from("1.0"),
        )?;
        assert_eq!(amt_out, 5_181_584_614, "Test case 2 failed");

        let amt_out = quote_swap_no_fees(
            10000000,          // 10 * 10^6
            1_000_000_000_000, // 1_000 * 10^9
            3_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            false,
            Decimal::from("2.0"),
            Decimal::from("1.0"),
        )?;
        assert_eq!(amt_out, 2_138_121_895, "Test case 3 failed");

        Ok(())
    }

    fn assert_get_d_u64(reserve_a: u64, reserve_b: u64, amp: u64, expected: u64) {
        assert_eq!(
            get_d(u256(reserve_a), u256(reserve_b), u256(amp)),
            u256(expected)
        );
    }
    fn assert_get_d(reserve_a: U256, reserve_b: U256, amp: U256, expected: U256) {
        assert_eq!(get_d(reserve_a, reserve_b, amp,), expected);
    }

    fn assert_get_y_u64(reserve_in: u64, amp: u64, d: u64, expected: u64) {
        assert_eq!(get_y(u256(reserve_in), u256(amp), u256(d)), u256(expected));
    }

    fn assert_get_y_scaled(reserve_in: U256, amp: U256, d: U256, expected: U256) {
        let upscale = U256::from(10u64).pow(U256::from(10u64));
        let result = get_y(reserve_in * upscale, amp, d * upscale) / upscale;
        let diff = if result > expected {
            result - expected
        } else {
            expected - result
        };
        assert!(
            diff <= U256::one(),
            "Difference too large: result = {}, expected = {}, diff = {}",
            result,
            expected,
            diff
        );
    }

    fn u256(val: u64) -> U256 {
        U256::from(val)
    }

    #[test]
    fn test_get_d() {
        assert_get_d_u64(1_000_000, 1_000_000, 20_000, 2_000_000);
        assert_get_d_u64(
            646_604_101_554_903,
            430_825_829_860_939,
            10_000,
            1_077_207_198_258_876,
        );
        assert_get_d_u64(
            208_391_493_399_283,
            381_737_267_304_454,
            6_000,
            589_673_027_554_751,
        );
        assert_get_d_u64(
            357_533_698_368_810,
            292_279_113_116_023,
            200_000,
            649_811_157_409_887,
        );
        assert_get_d_u64(
            640_219_149_077_469,
            749_346_581_809_482,
            6_000,
            1_389_495_058_454_884,
        );
        assert_get_d_u64(
            796_587_650_933_232,
            263_696_548_289_376,
            20_000,
            1_059_395_029_204_629,
        );
        assert_get_d_u64(
            645_814_702_742_123,
            941_346_843_035_970,
            6_000,
            1_586_694_700_461_120,
        );
        assert_get_d_u64(
            36_731_011_531_180,
            112_244_514_819_796,
            6_000,
            148_556_820_223_757,
        );
        assert_get_d_u64(
            638_355_455_638_005,
            144_419_816_425_350,
            20_000,
            781_493_318_669_443,
        );
        assert_get_d_u64(
            747_070_395_683_716,
            583_370_126_767_355,
            200_000,
            1_330_435_412_150_341,
        );
        assert_get_d_u64(
            222_152_880_197_132,
            503_754_962_483_370,
            10_000,
            725_272_897_710_721,
        );
    }

    #[test]
    fn test_get_d_scaled() {
        // Tests that scaling the reserves leads to the linear scaling of the D value
        let upscale = U256::from(10u64).pow(U256::from(10u64));

        assert_get_d(
            u256(1_000_000) * upscale,
            u256(1_000_000) * upscale,
            u256(20_000),
            u256(2_000_000) * upscale,
        );
        assert_eq!(
            get_d(
                u256(646_604_101_554_903) * upscale,
                u256(430_825_829_860_939) * upscale,
                u256(10_000)
            ) / upscale,
            u256(1_077_207_198_258_876)
        );
        assert_eq!(
            get_d(
                u256(208_391_493_399_283) * upscale,
                u256(381_737_267_304_454) * upscale,
                u256(6_000)
            ) / upscale,
            u256(589_673_027_554_751)
        );
        assert_eq!(
            get_d(
                u256(357_533_698_368_810) * upscale,
                u256(292_279_113_116_023) * upscale,
                u256(200_000)
            ) / upscale,
            u256(649_811_157_409_887)
        );
        assert_eq!(
            get_d(
                u256(640_219_149_077_469) * upscale,
                u256(749_346_581_809_482) * upscale,
                u256(6_000)
            ) / upscale,
            u256(1_389_495_058_454_884)
        );
        assert_eq!(
            get_d(
                u256(796_587_650_933_232) * upscale,
                u256(263_696_548_289_376) * upscale,
                u256(20_000)
            ) / upscale,
            u256(1_059_395_029_204_629)
        );
        assert_eq!(
            get_d(
                u256(645_814_702_742_123) * upscale,
                u256(941_346_843_035_970) * upscale,
                u256(6_000)
            ) / upscale,
            u256(1_586_694_700_461_120)
        );
        assert_eq!(
            get_d(
                u256(36_731_011_531_180) * upscale,
                u256(112_244_514_819_796) * upscale,
                u256(6_000)
            ) / upscale,
            u256(148_556_820_223_757)
        );
        assert_eq!(
            get_d(
                u256(638_355_455_638_005) * upscale,
                u256(144_419_816_425_350) * upscale,
                u256(20_000)
            ) / upscale,
            u256(781_493_318_669_443)
        );
        assert_eq!(
            get_d(
                u256(747_070_395_683_716) * upscale,
                u256(583_370_126_767_355) * upscale,
                u256(200_000)
            ) / upscale,
            u256(1_330_435_412_150_341)
        );
        assert_eq!(
            get_d(
                u256(222_152_880_197_132) * upscale,
                u256(503_754_962_483_370) * upscale,
                u256(10_000)
            ) / upscale,
            u256(725_272_897_710_721)
        );

        assert_get_d(
            u256(30_000_000_000_000),
            u256(10_000_000_000_000),
            u256(200),
            u256(38_041_326_932_308),
        );
    }

    #[test]
    fn test_get_y() {
        // Expected values generated from curve stable swap contract
        // D values are generated from the results of the previous test
        assert_get_y_u64(1_010_000, 20_000, 2_000_000, 990_000);
        assert_get_y_u64(
            1_045_311_940_606_135,
            10_000,
            1_077_207_198_258_876,
            54_125_279_774_978,
        );
        assert_get_y_u64(
            628_789_391_533_719,
            6_000,
            589_673_027_554_751,
            12_102_396_904_252,
        );
        assert_get_y_u64(
            664_497_701_537_459,
            200_000,
            649_811_157_409_887,
            1_571_656_363_072,
        );
        assert_get_y_u64(
            1_241_196_069_415_337,
            6_000,
            1_389_495_058_454_884,
            164_151_111_358_319,
        );
        assert_get_y_u64(
            1_207_464_631_415_294,
            20_000,
            1_059_395_029_204_629,
            3_978_315_032_067,
        );
        assert_get_y_u64(
            1_326_030_781_815_325,
            6_000,
            1_586_694_700_461_120,
            270_631_769_558_978,
        );
        assert_get_y_u64(
            596_549_235_149_733,
            6_000,
            148_556_820_223_757,
            25_485_695_510,
        );
        assert_get_y_u64(
            1_412_549_409_240_877,
            20_000,
            781_493_318_669_443,
            333_436_412_241,
        );
        assert_get_y_u64(
            966_973_926_501_573,
            200_000,
            1_330_435_412_150_341,
            363_547_559_872_801,
        );
        assert_get_y_u64(
            468_614_952_287_735,
            10_000,
            725_272_897_710_721,
            256_991_438_480_111,
        );

        // Simulating a trade: buy sui, sell 1000 usdc
        // sui reserve after trade: 2984.5303826098
        assert_get_y_u64(
            10_000_000_000_000u64 + 100_000_000_000u64,
            1_u64 * 2_u64 * (A_PRECISION as u64),
            38_041_326_932_308u64,
            29_845_303_826_098u64,
        );
        // usdc reserve after trade: 996
        assert_get_y_u64(
            30_000_000_000_000u64 + 51_565_391_310u64,
            1_u64 * 2_u64 * (A_PRECISION as u64),
            38_041_326_932_308u64,
            9_966_843_369_867u64,
        );
    }

    #[test]
    fn test_scaled_y() {
        // let upscale = U256::from(10u64).pow(U256::from(10u64));

        assert_get_y_scaled(
            u256(1_010_000),
            u256(20_000),
            u256(2_000_000),
            u256(990_000),
        );
        assert_get_y_scaled(
            u256(1_045_311_940_606_135),
            u256(10_000),
            u256(1_077_207_198_258_876),
            u256(54_125_279_774_978),
        );
        assert_get_y_scaled(
            u256(628_789_391_533_719),
            u256(6_000),
            u256(589_673_027_554_751),
            u256(12_102_396_904_252),
        );
        assert_get_y_scaled(
            u256(664_497_701_537_459),
            u256(200_000),
            u256(649_811_157_409_887),
            u256(1_571_656_363_072),
        );
        assert_get_y_scaled(
            u256(1_241_196_069_415_337),
            u256(6_000),
            u256(1_389_495_058_454_884),
            u256(164_151_111_358_319),
        );
        assert_get_y_scaled(
            u256(1_207_464_631_415_294),
            u256(20_000),
            u256(1_059_395_029_204_629),
            u256(3_978_315_032_067),
        );
        assert_get_y_scaled(
            u256(1_326_030_781_815_325),
            u256(6_000),
            u256(1_586_694_700_461_120),
            u256(270_631_769_558_978),
        );
        assert_get_y_scaled(
            u256(596_549_235_149_733),
            u256(6_000),
            u256(148_556_820_223_757),
            u256(25_485_695_510),
        );
        assert_get_y_scaled(
            u256(1_412_549_409_240_877),
            u256(20_000),
            u256(781_493_318_669_443),
            u256(333_436_412_241),
        );
        assert_get_y_scaled(
            u256(966_973_926_501_573),
            u256(200_000),
            u256(1_330_435_412_150_341),
            u256(363_547_559_872_801),
        );
        assert_get_y_scaled(
            u256(468_614_952_287_735),
            u256(10_000),
            u256(725_272_897_710_721),
            u256(256_991_438_480_111),
        );
    }
}
