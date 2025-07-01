import Decimal from "decimal.js";
import { FixedPoint64 } from "../math/fixedPoint64";
import { getQuote, SwapQuote } from "..";

// Swap function - with btoken amounts
export function quoteSwap(
  bTokenAmountIn: bigint,
  bTokenReserveX: bigint,
  bTokenReserveY: bigint,
  priceX: number,
  priceY: number,
  decimalsX: number,
  decimalsY: number,
  amplifier: number,
  x2y: boolean,
  bTokenRatioX: Decimal,
  bTokenRatioY: Decimal,
  swapFeeBps: bigint,
): SwapQuote {
  const amountOutBToken = quoteSwapNoFees(
    bTokenAmountIn,
    bTokenReserveX,
    bTokenReserveY,
    priceX,
    priceY,
    decimalsX,
    decimalsY,
    amplifier,
    x2y,
    bTokenRatioX,
    bTokenRatioY,
  );

  return getQuote(bTokenAmountIn, amountOutBToken, x2y, swapFeeBps);
}

// Swap function - with btoken amounts
export function quoteSwapNoFees(
  bTokenAmountIn: bigint,
  bTokenReserveX: bigint,
  bTokenReserveY: bigint,
  priceX: number,
  priceY: number,
  decimalsX: number,
  decimalsY: number,
  amplifier: number,
  x2y: boolean,
  bTokenRatioX: Decimal,
  bTokenRatioY: Decimal,
): bigint {
  bTokenRatioX = bTokenRatioX.toDecimalPlaces(18, 1);
  bTokenRatioY = bTokenRatioY.toDecimalPlaces(18, 1);

  const toUnderlying = (() => {
    return (btokenAmount: bigint, bTokenRatio: Decimal): bigint => {
      return BigInt(
        new Decimal(btokenAmount.toString())
          .mul(bTokenRatio)
          .trunc()
          .toString(),
      );
    };
  })();

  const toBToken = (() => {
    return (amount: bigint, bTokenRatio: Decimal): bigint => {
      return BigInt(
        new Decimal(amount.toString()).div(bTokenRatio).trunc().toString(),
      );
    };
  })();

  const reserveX = toUnderlying(bTokenReserveX, bTokenRatioX);
  const reserveY = toUnderlying(bTokenReserveY, bTokenRatioY);
  let amountOutUnderlying: bigint;
  let amountOutBToken: bigint;

  if (x2y) {
    const amountIn = toUnderlying(bTokenAmountIn, bTokenRatioX);

    amountOutUnderlying = quoteSwapInner(
      amountIn,
      reserveX,
      reserveY,
      priceX,
      priceY,
      decimalsX,
      decimalsY,
      amplifier,
      x2y,
    );

    amountOutBToken = toBToken(amountOutUnderlying, bTokenRatioY);

    if (amountOutBToken >= bTokenReserveY) {
      amountOutBToken = BigInt(0);
    }
  } else {
    const amountIn = toUnderlying(bTokenAmountIn, bTokenRatioY);

    amountOutUnderlying = quoteSwapInner(
      amountIn,
      reserveX,
      reserveY,
      priceX,
      priceY,
      decimalsX,
      decimalsY,
      amplifier,
      x2y,
    );

    amountOutBToken = toBToken(amountOutUnderlying, bTokenRatioX);

    if (amountOutBToken >= bTokenReserveX) {
      amountOutBToken = BigInt(0);
    }
  }

  return amountOutBToken;
}

// Swap function - with underlying amounts
export function quoteSwapInner(
  // amountIn (Underlying)
  amountIn: bigint,
  // reserveX (Underlying)
  reserveX: bigint,
  // reserveY (Underlying)
  reserveY: bigint,
  // priceX (Underlying)
  priceX: number,
  // priceY (Underlying)
  priceY: number,
  decimalsX: number,
  decimalsY: number,
  amplifier: number,
  x2y: boolean,
): bigint {
  // Convert inputs to FixedPoint64
  const rX = FixedPoint64.from(reserveX);
  const rY = FixedPoint64.from(reserveY);
  const pX = FixedPoint64.from(BigInt(priceX));
  const pY = FixedPoint64.from(BigInt(priceY));
  const amp = FixedPoint64.from(BigInt(amplifier));
  const deltaIn = FixedPoint64.from(amountIn);

  const price_raw = pX.div(pY);

  // Compute dec_pow based on decimals difference
  const dec_pow =
    decimalsX >= decimalsY
      ? FixedPoint64.from(BigInt(10)).pow(decimalsX - decimalsY)
      : FixedPoint64.one().div(
          FixedPoint64.from(BigInt(10)).pow(decimalsY - decimalsX),
        );

  // Compute k: trade utilization
  const k = x2y
    ? deltaIn.mul(price_raw).div(rY.mul(dec_pow))
    : deltaIn.mul(dec_pow).div(rX.mul(price_raw));

  // Compute z_upper_bound
  const max_bound = FixedPoint64.fromRational(
    BigInt(9999999999),
    BigInt(10000000000),
  );
  const initialZ = max_bound.lt(k) ? max_bound : k;

  // Compute z using Newton-Raphson
  const z = newtonRaphson(k, amp, initialZ);

  // Compute delta_out
  const deltaOut = (x2y ? z.mul(rY) : z.mul(rX)).toU128Down();

  // Check if trade depletes output reserve
  if (x2y) {
    if (deltaOut >= reserveY) {
      return BigInt(0);
    }
  } else {
    if (deltaOut >= reserveX) {
      return BigInt(0);
    }
  }

  return deltaOut;
}

// Newton-Raphson method
function newtonRaphson(
  k: FixedPoint64,
  a: FixedPoint64,
  initialZ: FixedPoint64,
): FixedPoint64 {
  const one = FixedPoint64.one();
  const minZ = FixedPoint64.fromRational(BigInt(1), BigInt(100000)); // 1e-5
  const maxZ = FixedPoint64.fromRational(
    BigInt("999999999999999999"),
    BigInt("1000000000000000000"),
  ); // 0.999999999999999999
  const tol = FixedPoint64.fromRational(BigInt(1), BigInt(100000000000000)); // 1e-10
  const max_iter = 20;

  // Improve initial guess
  let z = initialZ.gte(one) ? maxZ : initialZ;
  let i = 0;

  while (i < max_iter) {
    // Compute f(z)
    const [fxVal, fxPositive] = computeF(z, a, k);

    // Check convergence
    if (fxVal.lt(tol)) {
      break;
    }

    // Compute f'(z)
    const fp = computeFPrime(z, a);

    // Check for near-zero derivative
    if (fp.lt(FixedPoint64.fromRational(BigInt(1), BigInt(10000000000)))) {
      throw new Error("Derivative near zero (error code 1001)");
    }

    // Newton step: z_new = z - alpha * f(z)/f'(z)
    const fxDivFp = fxVal.div(fp);
    let alpha = one;
    let newZ = fxPositive ? z.sub(fxDivFp) : z.add(fxDivFp);

    // Check if z_new is outside valid range
    if (newZ.lte(FixedPoint64.zero()) || newZ.gte(one)) {
      alpha = FixedPoint64.fromRational(BigInt(1), BigInt(2)); // 0.5
      const dampedStep = fxDivFp.mul(alpha);
      newZ = fxPositive ? z.sub(dampedStep) : z.add(dampedStep);
      newZ = newZ.lt(minZ) ? minZ : newZ.gt(maxZ) ? maxZ : newZ;
    }

    // Check if step is too small
    const step_size = newZ.gte(z) ? newZ.sub(z) : z.sub(newZ);
    if (step_size.lt(tol)) {
      break;
    }

    z = newZ;
    i++;
  }

  return z;
}

/**
 * Computes the function f(z, a, k) as defined in the Move code.
 * @param z First FixedPoint64 input.
 * @param a Second FixedPoint64 input.
 * @param k Third FixedPoint64 input.
 * @returns A tuple containing the resulting FixedPoint64 value and a boolean indicating the branch taken.
 */
function computeF(
  z: FixedPoint64,
  a: FixedPoint64,
  k: FixedPoint64,
): [FixedPoint64, boolean] {
  const one = FixedPoint64.one();

  // 64 * ln(2) in FixedPoint64 format
  const ln2_64 = FixedPoint64.fromRawValue(BigInt("12786308645202655660")).mul(
    FixedPoint64.from(BigInt(64)),
  );

  // Step 1: Compute (1 - 1/A) * z (always positive)
  const oneDivA = one.div(a);
  const term1 = z.mul(one.sub(oneDivA)); // Term 1 is always positive

  // Step 2: Compute (1/A) * ln(1 - z)
  const oneMinusZ = one.sub(z);
  const lnPlus64ln2 = oneMinusZ.lnPlus64ln2();

  // Assert ln_plus_64ln2 <= ln2_64
  if (lnPlus64ln2.gt(ln2_64)) {
    throw new Error("Assertion failed: ln_plus_64ln2 > ln2_64 (code 999)");
  }

  // ln_magnitude is always negative
  const lnMagnitude = ln2_64.sub(lnPlus64ln2);

  // Compute (1/A) * |ln(1-z)| (magnitude is positive, sign follows ln(1-z))
  // Term 2 is always negative
  const term2Magnitude = oneDivA.mul(lnMagnitude);

  // Term 1 is always positive, term 2 is always negative, so this will always result in an addition
  // Intermediate magnitude is always positive
  const intermediateMagnitude = term1.add(term2Magnitude);

  // t1 - t2 > 0 (always)
  if (intermediateMagnitude.gte(k)) {
    // BRANCH 1: If t1 - t2 > 0 && >= k, subtract k to get positive value
    return [intermediateMagnitude.sub(k), true];
  } else {
    // BRANCH 2: If t1 - t2 > 0 && < k, subtract k from intermediate_magnitude to get positive value
    return [k.sub(intermediateMagnitude), false];
  }
}

// Compute f'(z) = 1 - 1/A + 1/(A * (1 - z))
function computeFPrime(z: FixedPoint64, a: FixedPoint64): FixedPoint64 {
  const one = FixedPoint64.one();
  const one_div_a = one.div(a);
  const term3 = one.div(a.mul(one.sub(z)));

  return one.sub(one_div_a).add(term3);
}
