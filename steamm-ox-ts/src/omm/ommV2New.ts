import Decimal from "decimal.js";
import { BPS_SCALE, getQuote, SwapQuote } from "..";

export const A_PRECISION = 100;
export const LIMIT = 255;
export const SCALE = BigInt(10000000000);

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
  priceConfidenceA: Decimal,
  priceConfidenceB: Decimal,
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

  const priceUncertaintyRatioA = priceUncertaintyRatio(
    new Decimal(priceX),
    priceConfidenceA,
  );
  const priceUncertaintyRatioB = priceUncertaintyRatio(
    new Decimal(priceY),
    priceConfidenceB,
  );

  return getQuote(
    bTokenAmountIn,
    amountOutBToken,
    x2y,
    swapFeeBps,
    BigInt(
      priceUncertaintyRatioA > priceUncertaintyRatioB
        ? priceUncertaintyRatioA
        : priceUncertaintyRatioB,
    ),
  );
}

export function priceUncertaintyRatio(
  price: Decimal,
  priceConfidence: Decimal,
): bigint {
  try {
    const result = priceConfidence.mul(new Decimal(10_000)).div(price).floor();
    return BigInt(result.toString());
  } catch (e) {
    throw new Error("priceUncertaintyRatio: Calculation failed");
  }
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
          .floor()
          .toString(),
      );
    };
  })();

  const toBToken = (() => {
    return (amount: bigint, bTokenRatio: Decimal): bigint => {
      return BigInt(
        new Decimal(amount.toString()).div(bTokenRatio).floor().toString(),
      );
    };
  })();

  const amountIn = toUnderlying(
    bTokenAmountIn,
    x2y ? bTokenRatioX : bTokenRatioY,
  );

  const reserveX = toUnderlying(bTokenReserveX, bTokenRatioX);
  const reserveY = toUnderlying(bTokenReserveY, bTokenRatioY);
  let amountOutBToken: bigint;

  const [priceXIntegerPart, priceXDecimalPartInverted] = splitPrice(
    new Decimal(priceX),
  );
  const [priceYIntegerPart, priceYDecimalPartInverted] = splitPrice(
    new Decimal(priceY),
  );

  const scaledUsdReserveX = (() => {
    const scaledReserve = reserveX * SCALE;
    const scaledReserveUsd = toUsd(
      scaledReserve,
      priceXIntegerPart,
      priceXDecimalPartInverted,
    );
    return scaledReserveUsd / BigInt(10 ** decimalsX);
  })();

  const scaledUsdReserveY = (() => {
    const scaledReserve = reserveY * SCALE;
    const scaledReserveUsd = toUsd(
      scaledReserve,
      priceYIntegerPart,
      priceYDecimalPartInverted,
    );
    return scaledReserveUsd / BigInt(10 ** decimalsY);
  })();

  const scaledAmp = BigInt(amplifier * 2) * BigInt(A_PRECISION);
  const d = getD(scaledUsdReserveX, scaledUsdReserveY, scaledAmp);

  const scaledAmountIn = amountIn * SCALE;

  if (x2y) {
    const scaledUsdAmountIn =
      toUsd(scaledAmountIn, priceXIntegerPart, priceXDecimalPartInverted) /
      BigInt(10 ** decimalsX);

    const scaledUsdReserveOutAfterTrade = getY(
      scaledUsdReserveX + scaledUsdAmountIn,
      scaledAmp,
      d,
    );

    const scaledReserveOutAfterTrade = fromUsd(
      scaledUsdReserveOutAfterTrade,
      priceYIntegerPart,
      priceYDecimalPartInverted,
    );

    const reserveOutAfterTrade =
      (scaledReserveOutAfterTrade * BigInt(10 ** decimalsY)) / SCALE;

    const amountOutUnderlying = reserveY - reserveOutAfterTrade;
    amountOutBToken = toBToken(amountOutUnderlying, bTokenRatioY);

    if (amountOutBToken > bTokenReserveY) {
      amountOutBToken = BigInt(0);
    }
  } else {
    const scaledUsdAmountIn =
      toUsd(scaledAmountIn, priceYIntegerPart, priceYDecimalPartInverted) /
      BigInt(10 ** decimalsY);

    const scaledUsdReserveOutAfterTrade = getY(
      scaledUsdReserveY + scaledUsdAmountIn,
      scaledAmp,
      d,
    );

    const scaledReserveOutAfterTrade = fromUsd(
      scaledUsdReserveOutAfterTrade,
      priceXIntegerPart,
      priceXDecimalPartInverted,
    );

    const reserveOutAfterTrade =
      (scaledReserveOutAfterTrade * BigInt(10 ** decimalsX)) / SCALE;

    const amountOutUnderlying = reserveX - reserveOutAfterTrade;
    amountOutBToken = toBToken(amountOutUnderlying, bTokenRatioX);

    if (amountOutBToken > bTokenReserveX) {
      amountOutBToken = BigInt(0);
    }
  }

  return amountOutBToken;
}

export function splitPrice(price: Decimal): [bigint, bigint?] {
  const priceIntegerPart = BigInt(price.floor().toString());
  const priceDecimalPart = price.sub(price.floor());

  if (priceDecimalPart.equals(0)) {
    return [priceIntegerPart, undefined];
  } else {
    const priceDecimalPartInverted = Decimal.div(1, priceDecimalPart).floor();
    return [priceIntegerPart, BigInt(priceDecimalPartInverted.toString())];
  }
}

// Converts a unit amount into a USD amount
export function toUsd(
  amount: bigint,
  priceIntegerPart: bigint,
  priceDecimalPartInverted?: bigint,
): bigint {
  if (priceDecimalPartInverted !== undefined) {
    return amount * priceIntegerPart + amount / priceDecimalPartInverted;
  } else {
    return amount * priceIntegerPart;
  }
}

// Converts a USD amount into a unit amount
export function fromUsd(
  usdAmount: bigint,
  priceIntegerPart: bigint,
  priceDecimalPartInverted?: bigint,
): bigint {
  if (priceDecimalPartInverted !== undefined) {
    return (
      (usdAmount * priceDecimalPartInverted) /
      (priceIntegerPart * priceDecimalPartInverted + BigInt(1))
    );
  } else {
    return usdAmount / priceIntegerPart;
  }
}

export function getD(
  reserveA: bigint,
  reserveB: bigint,
  amp: bigint, // amp is already scaled by A_PRECISION
): bigint {
  // D invariant calculation in non-overflowing integer operations
  // iteratively
  // A * sum(x_i) * n**n + D = A * D * n**n + D**(n+1) / (n**n * prod(x_i))
  //
  // Converging solution:
  // D[j+1] = (A * n**n * sum(x_i) - D[j]**(n+1) / (n**n prod(x_i))) / (A * n**n - 1)
  const A_PRECISION_BI = BigInt(A_PRECISION);
  const nCoins = BigInt(2);

  const sum = reserveA + reserveB;
  const ann = amp * nCoins;

  // initial guess
  let d = sum;
  let limit = LIMIT;

  while (limit > 0) {
    let dP = d;
    dP = (dP * d) / reserveA;
    dP = (dP * d) / reserveB;
    dP = dP / (nCoins * nCoins);

    const dPrev = d;

    // (Ann * S / A_PRECISION + D_P * nCoins) * D / ((Ann - A_PRECISION) * D / A_PRECISION + (nCoins + 1) * D_P)
    const numerator = ((ann * sum) / A_PRECISION_BI + dP * nCoins) * d;
    const denominator =
      ((ann - A_PRECISION_BI) * d) / A_PRECISION_BI + (nCoins + BigInt(1)) * dP;

    d = numerator / denominator;

    if (d > dPrev) {
      if (d - dPrev <= BigInt(1)) {
        return d;
      }
    } else {
      if (dPrev - d <= BigInt(1)) {
        return d;
      }
    }

    limit -= 1;
  }

  throw new Error("getD: Did not converge");
}

export function getY(reserveIn: bigint, amp: bigint, d: bigint): bigint {
  const ann = amp * BigInt(2);

  const sum = reserveIn;
  let c = (d * d) / (BigInt(2) * reserveIn);
  c = (c * d * BigInt(A_PRECISION)) / (ann * BigInt(2));

  const b = sum + (d * BigInt(A_PRECISION)) / ann;
  let yPrev = BigInt(0);
  let y = d;

  let limit = LIMIT;

  while (limit > 0) {
    yPrev = y;
    y = (y * y + c) / (BigInt(2) * y + b - d);

    if (y > yPrev) {
      if (y - yPrev <= BigInt(1)) {
        return y;
      }
    } else {
      if (yPrev - y <= BigInt(1)) {
        return y;
      }
    }

    limit -= 1;
  }

  throw new Error("getY: Did not converge");
}
