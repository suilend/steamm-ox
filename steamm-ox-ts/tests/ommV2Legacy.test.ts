/* eslint-disable */
import { beforeAll, describe, expect, it } from "bun:test";
// import ommV2Legacy from "../index.ts";
import { ommV2Legacy } from "../src/index";
import { BN } from "bn.js";
import Decimal from "decimal.js";

export async function test() {
  describe("test legacy ommv2 quoter", async () => {
    it("Test quote swap", () => {
      let amountOut: bigint;
      amountOut = ommV2Legacy.quoteSwapNoFees(
        new BN(10_000_000), // 10 * 10^6
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(1_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("1.0"),
        new Decimal("1.0"),
      );
      expect(amountOut).toBe(BigInt(3_327_783_945));

      amountOut = ommV2Legacy.quoteSwapNoFees(
        new BN(100_000_000), // 100 * 10^6
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(1_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("1.0"),
        new Decimal("1.0"),
      );

      expect(amountOut).toBe(BigInt(32_783_899_517));

      amountOut = ommV2Legacy.quoteSwapNoFees(
        new BN(10_000_000_000), // 10 * 10^9
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(1_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        true,
        new Decimal("1.0"),
        new Decimal("1.0"),
      );
      expect(amountOut).toBe(BigInt(29_554_466));

      amountOut = ommV2Legacy.quoteSwapNoFees(
        new BN(100_000_000_000), // 100 * 10^9
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(1_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        true,
        new Decimal("1.0"),
        new Decimal("1.0"),
      );
      expect(amountOut).toBe(BigInt(259_181_779));
    });

    it("Test swap with different btoken ratios", () => {
      // Test case 1
      let amtOut = ommV2Legacy.quoteSwapNoFees(
        new BN(10_000_000), // 10 * 10^6
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(1_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("1.0"),
        new Decimal("1.0"),
      );
      expect(amtOut).toBe(BigInt(3_327_783_945));

      // Changing btoken ratio of input token does not impact output
      amtOut = ommV2Legacy.quoteSwapNoFees(
        new BN(5_000_000), // 10 / btoken ratio y * 10^6
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(1_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("1.0"),
        new Decimal("2.0"),
      );
      expect(amtOut).toBe(BigInt(3_327_783_945));

      // Changing btoken ratio of output token DOES impact output
      amtOut = ommV2Legacy.quoteSwapNoFees(
        new BN(10_000_000), // 10 * 10^6
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(1_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("0.5"),
        new Decimal("1.0"),
      );
      expect(amtOut).toBe(BigInt(6_644_493_744));

      // Changing btoken ratio of output token DOES impact output
      amtOut = ommV2Legacy.quoteSwapNoFees(
        new BN(10_000_000), // 10 * 10^6
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(1_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("2.0"),
        new Decimal("1.0"),
      );
      expect(amtOut).toBe(BigInt(1_665_278_549));
    });
  });
}

test();
