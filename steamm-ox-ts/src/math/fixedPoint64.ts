// === Constants ===
const LN2: bigint = BigInt("12786308645202655660"); // ln(2) in fixed 64 representation
const MAX_U128: bigint = BigInt("340282366920938463463374607431768211455"); // 2^128 - 1

// === FixedPoint64 Class ===
export class FixedPoint64 {
  value: bigint;

  constructor(value: bigint) {
    if (value < 0 || value > MAX_U128) {
      throw new Error(`Value out of range: ${value}`);
    }
    this.value = value;
  }

  // === Public-View Functions ===
  getValue(): bigint {
    return this.value;
  }

  // === Convert Functions ===
  static from(value: bigint): FixedPoint64 {
    const scaledValue = value << BigInt(64);
    if (scaledValue > MAX_U128) {
      throw new Error(`Out of range: ${scaledValue}`);
    }
    return new FixedPoint64(scaledValue);
  }

  static one(): FixedPoint64 {
    return FixedPoint64.from(BigInt(1));
  }

  static zero(): FixedPoint64 {
    return FixedPoint64.from(BigInt(0));
  }

  static fromRawValue(value: bigint): FixedPoint64 {
    return new FixedPoint64(value);
  }

  static fromRational(numerator: bigint, denominator: bigint): FixedPoint64 {
    if (denominator === BigInt(0)) {
      throw new Error("Zero division");
    }
    const scaledNumerator = numerator << BigInt(64);
    const quotient = scaledNumerator / denominator;
    if (quotient === BigInt(0) && numerator !== BigInt(0)) {
      throw new Error("Out of range: result too small");
    }
    if (quotient > MAX_U128) {
      throw new Error("Out of range: result too large");
    }
    return new FixedPoint64(quotient);
  }

  toU128(): bigint {
    const flooredNum = this.toU128Down() << BigInt(64);
    const boundary = flooredNum + (BigInt(1) << BigInt(63));
    return this.value < boundary ? this.toU128Down() : this.toU128Up();
  }

  toU128Down(): bigint {
    return this.value >> BigInt(64);
  }

  toU128Up(): bigint {
    const flooredNum = this.toU128Down() << BigInt(64);
    if (this.value === flooredNum) {
      return flooredNum >> BigInt(64);
    }
    return (flooredNum + (BigInt(1) << BigInt(64))) >> BigInt(64);
  }

  // === Comparison Functions ===
  isZero(): boolean {
    return this.value === BigInt(0);
  }

  equals(other: FixedPoint64): boolean {
    return this.value === other.value;
  }

  lt(other: FixedPoint64): boolean {
    return this.value < other.value;
  }

  gt(other: FixedPoint64): boolean {
    return this.value > other.value;
  }

  lte(other: FixedPoint64): boolean {
    return this.value <= other.value;
  }

  gte(other: FixedPoint64): boolean {
    return this.value >= other.value;
  }

  static max(x: FixedPoint64, y: FixedPoint64): FixedPoint64 {
    return x.value > y.value ? x : y;
  }

  static min(x: FixedPoint64, y: FixedPoint64): FixedPoint64 {
    return x.value < y.value ? x : y;
  }

  // === Math Operations ===
  sub(other: FixedPoint64): FixedPoint64 {
    if (this.value < other.value) {
      throw new Error("Negative result");
    }
    return new FixedPoint64(this.value - other.value);
  }

  add(other: FixedPoint64): FixedPoint64 {
    const result = this.value + other.value;
    if (result > MAX_U128) {
      throw new Error("Out of range");
    }
    return new FixedPoint64(result);
  }

  mul(other: FixedPoint64): FixedPoint64 {
    const product = (this.value * other.value) >> BigInt(64);
    if (product > MAX_U128) {
      throw new Error("Multiplication overflow");
    }
    return new FixedPoint64(product);
  }

  div(other: FixedPoint64): FixedPoint64 {
    if (other.value === BigInt(0)) {
      throw new Error("Zero division");
    }
    const result = (this.value << BigInt(64)) / other.value;
    if (result > MAX_U128) {
      throw new Error("Division overflow");
    }
    return new FixedPoint64(result);
  }

  pow(exponent: number): FixedPoint64 {
    const result = powRaw(this.value, BigInt(exponent));
    if (result > MAX_U128) {
      throw new Error("Overflow in pow");
    }
    return new FixedPoint64(result);
  }

  log2Plus64(): FixedPoint64 {
    return log2_64(this.value);
  }

  lnPlus64ln2(): FixedPoint64 {
    const x = log2_64(this.value).value;
    const result = (x * LN2) >> BigInt(64);
    return FixedPoint64.fromRawValue(result);
  }
}

// === Private Helper Functions ===
function powRaw(x: bigint, n: bigint): bigint {
  let res = BigInt(1) << BigInt(64);
  let nMut = n;
  while (nMut !== BigInt(0)) {
    if (nMut & BigInt(1)) {
      res = (res * x) >> BigInt(64);
    }
    nMut = nMut >> BigInt(1);
    x = (x * x) >> BigInt(64);
  }
  return res;
}

function floorLog2(x: bigint): number {
  if (x === BigInt(0)) {
    throw new Error("Log of zero");
  }
  let res = 0;
  let xMut = x;
  let n = BigInt(64);
  while (n > BigInt(0)) {
    if (xMut >= BigInt(1) << n) {
      xMut = xMut >> n;
      res += Number(n);
    }
    n = n >> BigInt(1);
  }
  return res;
}

function log2_64(x: bigint): FixedPoint64 {
  let xMut = x;
  const integerPart = floorLog2(xMut);
  if (xMut >= BigInt(1) << BigInt(63)) {
    xMut = xMut >> BigInt(integerPart - 63);
  } else {
    xMut = xMut << BigInt(63 - integerPart);
  }
  let frac = BigInt(0);
  let delta = BigInt(1) << BigInt(63);
  while (delta !== BigInt(0)) {
    xMut = (xMut * xMut) >> BigInt(63);
    if (xMut >= BigInt(2) << BigInt(63)) {
      frac += delta;
      xMut = xMut >> BigInt(1);
    }
    delta = delta >> BigInt(1);
  }
  return FixedPoint64.fromRawValue((BigInt(integerPart) << BigInt(64)) + frac);
}
