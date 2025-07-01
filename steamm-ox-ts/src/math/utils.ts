const MAX_U64 = BigInt("18446744073709551615");

export function safeMulDivUp(x: bigint, y: bigint, z: bigint): bigint {
  if (z === BigInt(0)) {
    throw new Error("Division by zero");
  }
  const res = numDivideAndRoundUp(x * y, z);
  if (res > MAX_U64) {
    throw new Error("Math overflow");
  }
  return res;
}

function numDivideAndRoundUp(numerator: bigint, denominator: bigint): bigint {
  return (numerator + denominator - BigInt(1)) / denominator;
}
