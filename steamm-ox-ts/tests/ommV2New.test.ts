/* eslint-disable */
import { beforeAll, describe, expect, it } from "bun:test";
import { ommV2New } from "../src/index";
import { BN } from "bn.js";
import Decimal from "decimal.js";
import { A_PRECISION } from "../src/omm/ommV2New";

export async function test() {
  describe("test legacy ommv2 quoter", async () => {
    it("Test quote swap", () => {
      let amountOut: bigint;
      amountOut = ommV2New.quoteSwapNoFees(
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
      expect(amountOut).toBe(BigInt(5_156_539_130));

      amountOut = ommV2New.quoteSwapNoFees(
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

      expect(amountOut).toBe(BigInt(49_852_725_213));

      amountOut = ommV2New.quoteSwapNoFees(
        new BN(5_156_539_131), // 10 * 10^9
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
      expect(amountOut).toBe(BigInt(9_920_471));
    });

    it("Test swap with different btoken ratios", () => {
      let amountOut: bigint;
      // Test case 1
      amountOut = ommV2New.quoteSwapNoFees(
        new BN(11_000_000), // 10 * 10^6 * 1.1
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(3_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("1.0"),
        new Decimal("1.0").div(new Decimal("1.1")),
      );
      expect(amountOut).toBe(BigInt(3_437_018_128));

      // Test case 2
      amountOut = ommV2New.quoteSwapNoFees(
        new BN(10_000_000), // 10 * 10^6
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(3_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("0.5"),
        new Decimal("1.0"),
      );
      expect(amountOut).toBe(BigInt(5_181_584_614));

      // Test case 3
      amountOut = ommV2New.quoteSwapNoFees(
        new BN(10_000_000), // 10 * 10^6
        new BN(1_000_000_000_000), // 1_000 * 10^9
        new BN(3_000_000_000), // 1_000 * 10^6
        3,
        1,
        9,
        6,
        1,
        false,
        new Decimal("2.0"),
        new Decimal("1.0"),
      );
      expect(amountOut).toBe(BigInt(2_138_121_895));
    });

    it("Test getD function with various inputs", () => {
      // Helper for getD tests
      function assertGetD(
        reserveA: bigint,
        reserveB: bigint,
        amp: bigint,
        expected: bigint,
      ) {
        expect(ommV2New.getD(reserveA, reserveB, amp)).toBe(expected);
      }

      assertGetD(
        BigInt("1000000"),
        BigInt("1000000"),
        BigInt("20000"),
        BigInt("2000000"),
      );
      assertGetD(
        BigInt("646604101554903"),
        BigInt("430825829860939"),
        BigInt("10000"),
        BigInt("1077207198258876"),
      );
      assertGetD(
        BigInt("208391493399283"),
        BigInt("381737267304454"),
        BigInt("6000"),
        BigInt("589673027554751"),
      );
      assertGetD(
        BigInt("357533698368810"),
        BigInt("292279113116023"),
        BigInt("200000"),
        BigInt("649811157409887"),
      );
      assertGetD(
        BigInt("640219149077469"),
        BigInt("749346581809482"),
        BigInt("6000"),
        BigInt("1389495058454884"),
      );
      assertGetD(
        BigInt("796587650933232"),
        BigInt("263696548289376"),
        BigInt("20000"),
        BigInt("1059395029204629"),
      );
      assertGetD(
        BigInt("645814702742123"),
        BigInt("941346843035970"),
        BigInt("6000"),
        BigInt("1586694700461120"),
      );
      assertGetD(
        BigInt("36731011531180"),
        BigInt("112244514819796"),
        BigInt("6000"),
        BigInt("148556820223757"),
      );
      assertGetD(
        BigInt("638355455638005"),
        BigInt("144419816425350"),
        BigInt("20000"),
        BigInt("781493318669443"),
      );
      assertGetD(
        BigInt("747070395683716"),
        BigInt("583370126767355"),
        BigInt("200000"),
        BigInt("1330435412150341"),
      );
      assertGetD(
        BigInt("222152880197132"),
        BigInt("503754962483370"),
        BigInt("10000"),
        BigInt("725272897710721"),
      );
    });

    it("Test getD scaling property (linear scaling with reserves)", () => {
      // Helper for getD scaling tests
      function assertGetDScaled(
        reserveA: bigint,
        reserveB: bigint,
        amp: bigint,
        expected: bigint,
      ) {
        expect(ommV2New.getD(reserveA, reserveB, amp)).toBe(expected);
      }

      const upscale = BigInt("10000000000");

      // Scaling both reserves by 'upscale' should scale D by 'upscale'
      assertGetDScaled(
        BigInt("1000000") * upscale,
        BigInt("1000000") * upscale,
        BigInt("20000"),
        BigInt("2000000") * upscale,
      );
      expect(
        ommV2New.getD(
          BigInt("646604101554903") * upscale,
          BigInt("430825829860939") * upscale,
          BigInt("10000"),
        ) / upscale,
      ).toBe(BigInt("1077207198258876"));
      expect(
        ommV2New.getD(
          BigInt("208391493399283") * upscale,
          BigInt("381737267304454") * upscale,
          BigInt("6000"),
        ) / upscale,
      ).toBe(BigInt("589673027554751"));
      expect(
        ommV2New.getD(
          BigInt("357533698368810") * upscale,
          BigInt("292279113116023") * upscale,
          BigInt("200000"),
        ) / upscale,
      ).toBe(BigInt("649811157409887"));
      expect(
        ommV2New.getD(
          BigInt("640219149077469") * upscale,
          BigInt("749346581809482") * upscale,
          BigInt("6000"),
        ) / upscale,
      ).toBe(BigInt("1389495058454884"));
      expect(
        ommV2New.getD(
          BigInt("796587650933232") * upscale,
          BigInt("263696548289376") * upscale,
          BigInt("20000"),
        ) / upscale,
      ).toBe(BigInt("1059395029204629"));
      expect(
        ommV2New.getD(
          BigInt("645814702742123") * upscale,
          BigInt("941346843035970") * upscale,
          BigInt("6000"),
        ) / upscale,
      ).toBe(BigInt("1586694700461120"));
      expect(
        ommV2New.getD(
          BigInt("36731011531180") * upscale,
          BigInt("112244514819796") * upscale,
          BigInt("6000"),
        ) / upscale,
      ).toBe(BigInt("148556820223757"));
      expect(
        ommV2New.getD(
          BigInt("638355455638005") * upscale,
          BigInt("144419816425350") * upscale,
          BigInt("20000"),
        ) / upscale,
      ).toBe(BigInt("781493318669443"));
      expect(
        ommV2New.getD(
          BigInt("747070395683716") * upscale,
          BigInt("583370126767355") * upscale,
          BigInt("200000"),
        ) / upscale,
      ).toBe(BigInt("1330435412150341"));
      expect(
        ommV2New.getD(
          BigInt("222152880197132") * upscale,
          BigInt("503754962483370") * upscale,
          BigInt("10000"),
        ) / upscale,
      ).toBe(BigInt("725272897710721"));

      // Non-scaled test for completeness
      assertGetDScaled(
        BigInt("30000000000000"),
        BigInt("10000000000000"),
        BigInt("200"),
        BigInt("38041326932308"),
      );
    });

    it("Test splitPrice with integer and decimal prices", () => {
      // Integer price
      expect(ommV2New.splitPrice(new Decimal(5))).toEqual([
        BigInt(5),
        undefined,
      ]);
      // Decimal price
      const [intPart, decInv] = ommV2New.splitPrice(new Decimal("2.5"));
      expect(intPart).toBe(BigInt(2));
      expect(typeof decInv).toBe("bigint");
      // Check that the inverted decimal part is correct
      expect(Number(decInv!)).toBeCloseTo(2); // 1/0.5 = 2
    });

    it("Test toUsd and fromUsd round-trip", () => {
      const amount = BigInt(1000);
      const priceInt = BigInt(3);
      const priceDecInv = BigInt(2); // price = 3.5
      const usd = ommV2New.toUsd(amount, priceInt, priceDecInv);
      const back = ommV2New.fromUsd(usd, priceInt, priceDecInv);
      // Should be less than or equal due to flooring in fromUsd
      expect(back <= amount).toBe(true);
    });

    it("Test getY convergence and monotonicity", () => {
      // getY should converge and output should be less than d
      const reserveIn = BigInt("1000000");
      const amp = BigInt("20000");
      const d = BigInt("2000000");
      const y = ommV2New.getY(reserveIn, amp, d);
      expect(y < d).toBe(true);
      expect(y > BigInt(0)).toBe(true);
    });

    it("Test getY with expected values from stable swap contract", () => {
      function assertGetY(
        reserveIn: bigint,
        amp: bigint,
        d: bigint,
        expected: bigint,
      ) {
        expect(ommV2New.getY(BigInt(reserveIn), BigInt(amp), BigInt(d))).toBe(
          BigInt(expected),
        );
      }

      assertGetY(1_010_000n, 20_000n, 2_000_000n, 990_000n);
      assertGetY(
        1_045_311_940_606_135n,
        10_000n,
        1_077_207_198_258_876n,
        54_125_279_774_978n,
      );
      assertGetY(
        628_789_391_533_719n,
        6_000n,
        589_673_027_554_751n,
        12_102_396_904_252n,
      );
      assertGetY(
        664_497_701_537_459n,
        200_000n,
        649_811_157_409_887n,
        1_571_656_363_072n,
      );
      assertGetY(
        1_241_196_069_415_337n,
        6_000n,
        1_389_495_058_454_884n,
        164_151_111_358_319n,
      );
      assertGetY(
        1_207_464_631_415_294n,
        20_000n,
        1_059_395_029_204_629n,
        3_978_315_032_067n,
      );
      assertGetY(
        1_326_030_781_815_325n,
        6_000n,
        1_586_694_700_461_120n,
        270_631_769_558_978n,
      );
      assertGetY(
        596_549_235_149_733n,
        6_000n,
        148_556_820_223_757n,
        25_485_695_510n,
      );
      assertGetY(
        1_412_549_409_240_877n,
        20_000n,
        781_493_318_669_443n,
        333_436_412_241n,
      );
      assertGetY(
        966_973_926_501_573n,
        200_000n,
        1_330_435_412_150_341n,
        363_547_559_872_801n,
      );
      assertGetY(
        468_614_952_287_735n,
        10_000n,
        725_272_897_710_721n,
        256_991_438_480_111n,
      );

      // Simulating a trade: buy sui, sell 1000 usdc
      // sui reserve after trade: 2984.5303826098
      assertGetY(
        10_000_000_000_000n + 100_000_000_000n,
        1n * 2n * BigInt(A_PRECISION),
        38_041_326_932_308n,
        29_845_303_826_098n,
      );
      // usdc reserve after trade: 996
      assertGetY(
        30_000_000_000_000n + 51_565_391_310n,
        1n * 2n * BigInt(A_PRECISION),
        38_041_326_932_308n,
        9_966_843_369_867n,
      );
    });

    it("Test getY scaling property (linear scaling with reserves)", () => {
      function assertGetYScaled(
        reserveIn: bigint,
        amp: bigint,
        d: bigint,
        expected: bigint,
      ) {
        const upscale = BigInt("10000000000");
        const result =
          ommV2New.getY(reserveIn * upscale, amp, d * upscale) / upscale;
        const diff = result > expected ? result - expected : expected - result;
        expect(diff <= BigInt(1)).toBe(true);
      }

      assertGetYScaled(
        BigInt("1010000"),
        BigInt("20000"),
        BigInt("2000000"),
        BigInt("990000"),
      );
      assertGetYScaled(
        BigInt("1045311940606135"),
        BigInt("10000"),
        BigInt("1077207198258876"),
        BigInt("54125279774978"),
      );
      assertGetYScaled(
        BigInt("628789391533719"),
        BigInt("6000"),
        BigInt("589673027554751"),
        BigInt("12102396904252"),
      );
      assertGetYScaled(
        BigInt("664497701537459"),
        BigInt("200000"),
        BigInt("649811157409887"),
        BigInt("1571656363072"),
      );
      assertGetYScaled(
        BigInt("1241196069415337"),
        BigInt("6000"),
        BigInt("1389495058454884"),
        BigInt("164151111358319"),
      );
      assertGetYScaled(
        BigInt("1207464631415294"),
        BigInt("20000"),
        BigInt("1059395029204629"),
        BigInt("3978315032067"),
      );
      assertGetYScaled(
        BigInt("1326030781815325"),
        BigInt("6000"),
        BigInt("1586694700461120"),
        BigInt("270631769558978"),
      );
      assertGetYScaled(
        BigInt("596549235149733"),
        BigInt("6000"),
        BigInt("148556820223757"),
        BigInt("25485695510"),
      );
      assertGetYScaled(
        BigInt("1412549409240877"),
        BigInt("20000"),
        BigInt("781493318669443"),
        BigInt("333436412241"),
      );
      assertGetYScaled(
        BigInt("966973926501573"),
        BigInt("200000"),
        BigInt("1330435412150341"),
        BigInt("363547559872801"),
      );
      assertGetYScaled(
        BigInt("468614952287735"),
        BigInt("10000"),
        BigInt("725272897710721"),
        BigInt("256991438480111"),
      );
    });
  });
}

test();
