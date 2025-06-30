use crate::{
    SwapQuote, get_quote,
    math::{decimal::Decimal, decimal_to_fixedpoint64, fixed_point::FixedPoint64},
    to_b_token, to_underlying,
};
use anyhow::Result;

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

    Ok(get_quote(
        b_token_amount_in,
        amount_out_btoken,
        x2y,
        swap_fee_bps,
        None,
    ))
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
    let reserve_x = to_underlying(b_token_reserve_x, &b_token_ratio_x);
    let reserve_y = to_underlying(b_token_reserve_y, &b_token_ratio_y);

    let (_, amount_out_b_token) = if x2y {
        let amount_in = to_underlying(b_token_amount_in, &b_token_ratio_x);
        let out = quote_swap_inner(
            amount_in as u128,
            reserve_x as u128,
            reserve_y as u128,
            price_x,
            price_y,
            decimals_x,
            decimals_y,
            amplifier,
            x2y,
        )?;
        let b_token = to_b_token(out as u64, &b_token_ratio_y);
        (out, b_token)
    } else {
        let amount_in = to_underlying(b_token_amount_in, &b_token_ratio_y);
        let out = quote_swap_inner(
            amount_in as u128,
            reserve_x as u128,
            reserve_y as u128,
            price_x,
            price_y,
            decimals_x,
            decimals_y,
            amplifier,
            x2y,
        )?;
        let b_token = to_b_token(out as u64, &b_token_ratio_x);
        (out, b_token)
    };

    if x2y && amount_out_b_token >= b_token_reserve_y {
        Ok(0)
    } else if !x2y && amount_out_b_token >= b_token_reserve_x {
        Ok(0)
    } else {
        Ok(amount_out_b_token)
    }
}

pub fn quote_swap_inner(
    // Amount in (underlying token - e.g. SUI or USDC)
    amount_in: u128,
    // Reserve X (underlying token - e.g. SUI)
    reserve_x: u128,
    // Reserve Y (underlying token - e.g. USDC)
    reserve_y: u128,
    price_x: Decimal,
    price_y: Decimal,
    decimals_x: u32,
    decimals_y: u32,
    amplifier: u32,
    x2y: bool,
) -> Result<u128> {
    let r_x = FixedPoint64::from(reserve_x)?;
    let r_y = FixedPoint64::from(reserve_y)?;
    let p_x = decimal_to_fixedpoint64(price_x)?;
    let p_y = decimal_to_fixedpoint64(price_y)?;
    let amp = FixedPoint64::from(amplifier as u128)?;
    let delta_in = FixedPoint64::from(amount_in)?;

    let dec_pow = if decimals_x >= decimals_y {
        FixedPoint64::from(10)?.pow(decimals_x - decimals_y)?
    } else {
        FixedPoint64::one()?.div(&FixedPoint64::from(10)?.pow(decimals_y - decimals_x)?)?
    };

    let k = if x2y {
        FixedPoint64::multiply_divide(&mut vec![delta_in, p_x], &mut vec![r_y, p_y, dec_pow])?
    } else {
        FixedPoint64::multiply_divide(&mut vec![delta_in, dec_pow, p_y], &mut vec![r_x, p_x])?
    };

    let max_bound = FixedPoint64::from_rational(9_999_999_999, 10_000_000_000)?;
    let initial_z = if max_bound.lt(&k) { max_bound } else { k };

    let z = newton_raphson(&k, &amp, &initial_z)?;

    let delta_out = if x2y {
        z.mul(&r_y)?.to_u128_down()
    } else {
        z.mul(&r_x)?.to_u128_down()
    };

    if x2y && delta_out >= reserve_y {
        Ok(0)
    } else if !x2y && delta_out >= reserve_x {
        Ok(0)
    } else {
        Ok(delta_out)
    }
}

fn newton_raphson(
    k: &FixedPoint64,
    a: &FixedPoint64,
    initial_z: &FixedPoint64,
) -> Result<FixedPoint64> {
    let one = FixedPoint64::one()?;
    let min_z = FixedPoint64::from_rational(1, 100_000)?; // 1e-5
    let max_z = FixedPoint64::from_rational(999_999_999_999_999_999, 1_000_000_000_000_000_000)?; // 0.999999999999999999
    let tol = FixedPoint64::from_rational(1, 100_000_000_000_000)?; // 1e-14
    let max_iter = 20;

    let mut z = if initial_z.gte(&one) {
        max_z
    } else {
        *initial_z
    };
    let mut i = 0;

    while i < max_iter {
        let (fx_val, fx_positive) = compute_f(&z, a, k)?;

        if fx_val.lt(&tol) {
            break;
        }

        let fp = compute_f_prime(&z, a)?;

        if fp.lt(&FixedPoint64::from_rational(1, 10_000_000_000)?) {
            return Err(anyhow::anyhow!("Derivative near zero (error code 1001)"));
        }

        let fx_div_fp = fx_val.div(&fp)?;
        let alpha = if fx_div_fp.gte(&one) {
            FixedPoint64::from_rational(1, 2)? // 0.5
        } else {
            one
        };
        let step = fx_div_fp.mul(&alpha)?;
        let new_z = if fx_positive {
            z.sub(&step)?
        } else {
            z.add(&step)?
        };

        let new_z = if new_z.lte(&FixedPoint64::zero()?) || new_z.gte(&one) {
            let damped_step = fx_div_fp.mul(&FixedPoint64::from_rational(1, 2)?)?;
            let temp_z = if fx_positive {
                z.sub(&damped_step)?
            } else {
                z.add(&damped_step)?
            };
            if temp_z.lt(&min_z) {
                min_z
            } else if temp_z.gt(&max_z) {
                max_z
            } else {
                temp_z
            }
        } else {
            new_z
        };

        let step_size = if new_z.gte(&z) {
            new_z.sub(&z)?
        } else {
            z.sub(&new_z)?
        };
        if step_size.lt(&tol) {
            break;
        }

        z = new_z;
        i += 1;
    }

    Ok(z)
}

fn compute_f(z: &FixedPoint64, a: &FixedPoint64, k: &FixedPoint64) -> Result<(FixedPoint64, bool)> {
    let one = FixedPoint64::one()?;
    let ln2_64 =
        FixedPoint64::from_raw_value(12_786_308_645_202_655_660)?.mul(&FixedPoint64::from(64)?)?;

    let one_div_a = one.div(a)?;
    let term1 = z.mul(&one.sub(&one_div_a)?)?; // Positive

    let one_minus_z = one.sub(z)?;
    let ln_plus_64ln2 = one_minus_z.ln_plus_64ln2()?;

    if ln_plus_64ln2.gt(&ln2_64) {
        return Err(anyhow::anyhow!("ln_plus_64ln2 > ln2_64 (code 999)"));
    }

    let ln_magnitude = ln2_64.sub(&ln_plus_64ln2)?;
    let term2_magnitude = one_div_a.mul(&ln_magnitude)?;

    let intermediate_magnitude = term1.add(&term2_magnitude)?;

    if intermediate_magnitude.gte(k) {
        Ok((intermediate_magnitude.sub(k)?, true))
    } else {
        Ok((k.sub(&intermediate_magnitude)?, false))
    }
}

fn compute_f_prime(z: &FixedPoint64, a: &FixedPoint64) -> Result<FixedPoint64> {
    let one = FixedPoint64::one()?;
    let one_div_a = one.div(a)?;
    let term3 = one.div(&a.mul(&one.sub(z)?)?)?;
    one.sub(&one_div_a)?.add(&term3)
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
        assert_eq!(amt_out, 3_327_783_945, "Test case 1 failed");

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
        assert_eq!(amt_out, 32_783_899_517, "Test case 2 failed");

        // Test case 3
        let amt_out = quote_swap_no_fees(
            10_000_000_000,    // 10 * 10^9
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
        assert_eq!(amt_out, 29_554_466, "Test case 3 failed");

        // Test case 4
        let amt_out = quote_swap_no_fees(
            100_000_000_000,   // 100 * 10^9
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
        assert_eq!(amt_out, 259_181_779, "Test case 4 failed");

        Ok(())
    }

    #[test]
    fn test_quote_swap_with_different_btoken_ratios() -> Result<()> {
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
        assert_eq!(amt_out, 3_327_783_945, "Test case 1 failed");

        // Changing btoken ratio of input token does not impact output
        let amt_out = quote_swap_no_fees(
            5_000_000,         // 10 / btoken ratio y * 10^6
            1_000_000_000_000, // 1_000 * 10^9
            1_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            false,
            Decimal::from("1.0"),
            Decimal::from("2.0"),
        )?;
        assert_eq!(amt_out, 3_327_783_945, "Test case 1 failed");

        // Changing btoken ratio of output token DOES impact output
        let amt_out = quote_swap_no_fees(
            10_000_000,        // 10 / * 10^6
            1_000_000_000_000, // 1_000 * 10^9
            1_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            false,
            Decimal::from("0.5"),
            Decimal::from("1.0"),
        )?;
        assert_eq!(amt_out, 6_644_493_744, "Test case 1 failed");

        // Changing btoken ratio of output token DOES impact output
        let amt_out = quote_swap_no_fees(
            10_000_000,        // 10 / * 10^6
            1_000_000_000_000, // 1_000 * 10^9
            1_000_000_000,     // 1_000 * 10^6
            Decimal::from("3"),
            Decimal::from("1"),
            9,
            6,
            1,
            false,
            Decimal::from("2.0"),
            Decimal::from("1.0"),
        )?;
        assert_eq!(amt_out, 1665278549, "Test case 1 failed");

        Ok(())
    }

    #[test]
    fn test_quote_swap_2() {
        let inputs = vec![
            // Input 1
            (
                4_644_540_u128,  // amount_in
                90_000_000_u128, // reserve_x
                8_100_000_u128,  // reserve_y
                4,               // price_x
                9,               // price_y
                7,               // decimals_x
                5,               // decimals_y
                100,             // amplifier
                false,           // x2y
            ),
            // Input 2
            (
                324_894_000_000_u128,
                258_000_u128,
                626_000_000_000_u128,
                6,
                9,
                3,
                9,
                2,
                false,
            ),
            // Input 3
            (
                6_855_840_u128,
                45_900_000_u128,
                62_100_000_u128,
                4,
                8,
                5,
                5,
                10,
                false,
            ),
            // Input 4
            (
                665_993_800_000_u128,
                3_610_000_u128,
                877_000_000_000_u128,
                2,
                3,
                4,
                9,
                2,
                false,
            ),
            // Input 5
            (
                7_879_648_u128,
                6_450_000_u128,
                8_120_000_u128,
                10,
                5,
                4,
                4,
                100,
                false,
            ),
            // Input 6
            (
                61_223_u128,
                750_000_000_u128,
                978_000_u128,
                3,
                5,
                7,
                3,
                1000,
                false,
            ),
            // Input 7
            (
                10_550_880_u128,
                405_000_000_000_u128,
                20_400_000_u128,
                3,
                4,
                9,
                5,
                100,
                false,
            ),
            // Input 8
            (
                1_678_644_000_u128,
                6_000_000_u128,
                4_860_000_000_u128,
                10,
                3,
                5,
                7,
                2,
                true,
            ),
            // Input 9
            (
                603_924_000_000_u128,
                414_000_000_000_u128,
                885_000_000_000_u128,
                3,
                2,
                9,
                9,
                100,
                false,
            ),
            // Input 10
            (
                945_096_000_u128,
                293_000_000_000_u128,
                6_360_000_000_u128,
                3,
                4,
                9,
                7,
                8000,
                false,
            ),
            // Input 11
            (
                8_527_520_u128,
                38_100_000_000_u128,
                95_600_000_u128,
                5,
                8,
                8,
                5,
                2,
                false,
            ),
            // Input 12
            (
                45_084_600_000_u128,
                648_000_000_u128,
                69_000_000_000_u128,
                3,
                7,
                6,
                9,
                2,
                true,
            ),
            // Input 13
            (
                6_791_584_000_u128,
                46_900_000_u128,
                7_460_000_000_u128,
                5,
                5,
                5,
                7,
                10,
                true,
            ),
            // Input 14
            (
                534_653_400_u128,
                42_000_000_u128,
                901_000_000_u128,
                8,
                3,
                5,
                6,
                1000,
                true,
            ),
            // Input 15
            (
                12_247_200_u128,
                349_000_000_u128,
                18_000_000_u128,
                8,
                5,
                6,
                6,
                1000,
                false,
            ),
            // Input 16
            (
                677_100_000_u128,
                4_240_000_000_u128,
                3_700_000_000_u128,
                4,
                8,
                7,
                7,
                1000,
                false,
            ),
            // Input 17
            (
                3_746_862_000_u128,
                37_500_000_000_u128,
                3_930_000_000_u128,
                9,
                6,
                8,
                7,
                1,
                true,
            ),
            // Input 18
            (
                5_891_520_u128,
                933_000_000_u128,
                72_200_000_u128,
                6,
                6,
                6,
                5,
                8000,
                true,
            ),
            // Input 19
            (
                1_870_480_000_u128,
                240_000_u128,
                4_120_000_000_u128,
                10,
                9,
                4,
                7,
                2,
                true,
            ),
            // Input 20
            (
                6_120_000_u128,
                72_400_000_000_u128,
                72_000_000_u128,
                8,
                9,
                8,
                5,
                1000,
                true,
            ),
            // Input 21
            (
                130_203_000_000_u128,
                447_000_000_000_u128,
                555_000_000_000_u128,
                10,
                5,
                9,
                9,
                1,
                false,
            ),
            // Input 22
            (
                77_616_000_000_u128,
                498_000_u128,
                660_000_000_000_u128,
                1,
                7,
                3,
                9,
                1,
                false,
            ),
            // Input 23
            (
                377_348_400_000_u128,
                913_000_000_000_u128,
                733_000_000_000_u128,
                4,
                8,
                9,
                9,
                1000,
                false,
            ),
            // Input 24
            (
                77_398_240_000_u128,
                692_000_u128,
                77_600_000_000_u128,
                2,
                2,
                3,
                8,
                1000,
                true,
            ),
        ];

        let expected_results = vec![
            89_999_999_u64,
            242_872_u64,
            13_464_431_u64,
            3_571_668_u64,
            3_918_681_u64,
            749_999_999_u64,
            140_358_664_628_u64,
            4_859_999_999_u64,
            394_009_395_174_u64,
            126_007_959_450_u64,
            12_354_540_449_u64,
            68_999_999_999_u64,
            7_459_999_999_u64,
            900_999_999_u64,
            7_654_414_u64,
            1_353_922_942_u64,
            523_690_572_u64,
            589_151_u64,
            4_119_999_999_u64,
            5_439_u64,
            60_582_784_461_u64,
            330_729_u64,
            753_855_713_528_u64,
            77_599_999_999_u64,
        ];

        let mut results = Vec::new();

        for (i, input) in inputs.iter().enumerate() {
            let result = quote_swap_no_fees(
                input.0 as u64,                // amount_in
                input.1 as u64,                // reserve_x
                input.2 as u64,                // reserve_y
                Decimal::from(input.3 as u64), // price_x
                Decimal::from(input.4 as u64), // price_y
                input.5,                       // decimals_x
                input.6,                       // decimals_y
                input.7,                       // amplifier
                input.8,                       // x2y
                Decimal::from("1.0"),          // btoken ratio x
                Decimal::from("1.0"),          // btoken ratio y
            );

            match result {
                Ok(amt_out) => {
                    results.push(amt_out);
                }
                Err(e) => {
                    panic!("Error at iteration {}: {:?}", i + 1, e);
                }
            }
        }

        for (i, (result, expected)) in results.iter().zip(expected_results.iter()).enumerate() {
            assert_eq!(
                result,
                expected,
                "Test failed at iteration {}: expected {}, got {}",
                i + 1,
                expected,
                result
            );
        }
    }
}
