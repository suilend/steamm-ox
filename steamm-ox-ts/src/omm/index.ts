import Decimal from "decimal.js";
import { ommV2Legacy, ommV2New, SwapQuote } from "..";

export * as ommV2Legacy from "./ommV2Legacy";
export * as ommV2New from "./ommV2New";

export enum QuoterType {
  Ommv2Legacy = "Ommv2Legacy",
  Ommv2New = "Ommv2New",
}

export class SteammPool {
  bTokenReserveX: bigint;
  bTokenReserveY: bigint;
  decimalsX: number;
  decimalsY: number;
  amplifier: number;
  swapFeeBps: bigint;
  quoterType: QuoterType;

  constructor(
    bTokenReserveX: bigint,
    bTokenReserveY: bigint,
    decimalsX: number,
    decimalsY: number,
    amplifier: number,
    swapFeeBps: bigint,
    quoterType: QuoterType,
  ) {
    this.bTokenReserveX = bTokenReserveX;
    this.bTokenReserveY = bTokenReserveY;
    this.decimalsX = decimalsX;
    this.decimalsY = decimalsY;
    this.amplifier = amplifier;
    this.swapFeeBps = swapFeeBps;
    this.quoterType = quoterType;
  }

  quoteSwap(
    bTokenAmountIn: bigint,
    priceX: number,
    priceY: number,
    x2y: boolean,
    bTokenRatioX: Decimal,
    bTokenRatioY: Decimal,
    priceConfidenceA?: Decimal,
    priceConfidenceB?: Decimal,
  ): SwapQuote {
    switch (this.quoterType) {
      case QuoterType.Ommv2Legacy:
        return ommV2Legacy.quoteSwap(
          bTokenAmountIn,
          this.bTokenReserveX,
          this.bTokenReserveY,
          priceX,
          priceY,
          this.decimalsX,
          this.decimalsY,
          this.amplifier,
          x2y,
          bTokenRatioX,
          bTokenRatioY,
          this.swapFeeBps,
        );
      case QuoterType.Ommv2New:
        if (priceConfidenceA === undefined || priceConfidenceB === undefined) {
          throw new Error(
            "priceConfidenceA and priceConfidenceB are required for Ommv2",
          );
        }
        return ommV2New.quoteSwap(
          bTokenAmountIn,
          this.bTokenReserveX,
          this.bTokenReserveY,
          priceX,
          priceY,
          this.decimalsX,
          this.decimalsY,
          this.amplifier,
          x2y,
          bTokenRatioX,
          bTokenRatioY,
          this.swapFeeBps,
          priceConfidenceA,
          priceConfidenceB,
        );
      default:
        throw new Error("Unknown quoter type");
    }
  }
}
