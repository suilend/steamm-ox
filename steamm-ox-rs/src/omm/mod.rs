use crate::{SwapQuote, math::decimal::Decimal};
use anyhow::Result;

pub mod omm_v2_legacy;
pub mod omm_v2_new;

pub struct SteammPool {
    pub b_token_reserve_x: u64,
    pub b_token_reserve_y: u64,
    pub decimals_x: u32,
    pub decimals_y: u32,
    pub amplifier: u32,
    pub swap_fee_bps: u64,
    pub quoter_type: QuoterType,
}

pub enum QuoterType {
    Ommv2Legacy,
    Ommv2,
}

impl SteammPool {
    pub fn new(
        b_token_reserve_x: u64,
        b_token_reserve_y: u64,
        decimals_x: u32,
        decimals_y: u32,
        amplifier: u32,
        swap_fee_bps: u64,
        quoter_type: QuoterType,
    ) -> Self {
        Self {
            b_token_reserve_x,
            b_token_reserve_y,
            decimals_x,
            decimals_y,
            amplifier,
            swap_fee_bps,
            quoter_type,
        }
    }

    pub fn quote_swap(
        &self,
        b_token_amount_in: u64,
        price_x: Decimal,
        price_y: Decimal,
        x2y: bool,
        b_token_ratio_x: Decimal,
        b_token_ratio_y: Decimal,
        price_confidence_a: Option<Decimal>,
        price_confidence_b: Option<Decimal>,
    ) -> Result<SwapQuote> {
        match self.quoter_type {
            QuoterType::Ommv2Legacy => omm_v2_legacy::quote_swap(
                b_token_amount_in,
                self.b_token_reserve_x,
                self.b_token_reserve_y,
                price_x.clone(),
                price_y.clone(),
                self.decimals_x,
                self.decimals_y,
                self.amplifier,
                x2y,
                b_token_ratio_x,
                b_token_ratio_y,
                self.swap_fee_bps,
            ),
            QuoterType::Ommv2 => omm_v2_new::quote_swap(
                b_token_amount_in,
                self.b_token_reserve_x,
                self.b_token_reserve_y,
                price_x.clone(),
                price_y.clone(),
                self.decimals_x,
                self.decimals_y,
                self.amplifier,
                x2y,
                b_token_ratio_x,
                b_token_ratio_y,
                self.swap_fee_bps,
                price_confidence_a.unwrap(),
                price_confidence_b.unwrap(),
            ),
        }
    }
}
