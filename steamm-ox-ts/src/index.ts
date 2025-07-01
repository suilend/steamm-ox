import { safeMulDivUp } from "./math/utils";

export * from "./math/fixedPoint64";
export * as ommV2Legacy from "./omm/ommV2Legacy";
export * as ommV2New from "./omm/ommV2New";

const PROTOCOL_FEE_NUMERATOR = BigInt(200);
export const BPS_SCALE = BigInt(10_000);

export interface SwapQuote {
  amount_in: bigint;
  amount_out: bigint;
  protocol_fees: bigint;
  pool_fees: bigint;
  a2b: boolean;
}

export function computeSwapFees(
  amount: bigint,
  swapFeeBps: bigint,
  swapFeeOverrideNumerator?: bigint,
): [bigint, bigint] {
  const protocolFeeNum = PROTOCOL_FEE_NUMERATOR;
  const protocolFeeDenom = BPS_SCALE;

  let poolFeeNum: bigint, poolFeeDenom: bigint;
  if (swapFeeOverrideNumerator !== undefined) {
    const poolFeeNumDefault = swapFeeBps;
    const poolFeeDenomDefault = BPS_SCALE;
    if (
      swapFeeOverrideNumerator * poolFeeDenomDefault >
      poolFeeNumDefault * BPS_SCALE
    ) {
      poolFeeNum = swapFeeOverrideNumerator;
      poolFeeDenom = BPS_SCALE;
    } else {
      poolFeeNum = poolFeeNumDefault;
      poolFeeDenom = poolFeeDenomDefault;
    }
  } else {
    poolFeeNum = swapFeeBps;
    poolFeeDenom = BPS_SCALE;
  }

  const totalFees = safeMulDivUp(amount, poolFeeNum, poolFeeDenom);
  const protocolFees = safeMulDivUp(
    totalFees,
    protocolFeeNum,
    protocolFeeDenom,
  );
  const poolFees = totalFees - protocolFees;

  return [protocolFees, poolFees];
}

export function getQuote(
  amount_in: bigint,
  amount_out: bigint,
  a2b: boolean,
  swap_fee_bps: bigint,
  swap_fee_override_numerator?: bigint,
): SwapQuote {
  const [protocol_fees, pool_fees] = computeSwapFees(
    amount_out,
    swap_fee_bps,
    swap_fee_override_numerator,
  );
  const amount_out_net =
    amount_out - protocol_fees - pool_fees > BigInt(0)
      ? amount_out - protocol_fees - pool_fees
      : BigInt(0);

  return {
    amount_in,
    amount_out: amount_out_net,
    protocol_fees,
    pool_fees,
    a2b,
  };
}
