use agneta_core::{BookRegime, FixedPrice, FixedQ32_32, NanoTime, Volume};
use crate::level::PriceLevel;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TickMetrics {
    pub timestamp: NanoTime,
    pub delta_ns: u64,
    pub micro_price: FixedPrice,
    pub spread: FixedPrice,
    pub ofi_l1: i64,
    pub ofi_deep: i64,
    pub book_imbalance_bps: i64,
    pub locked_limit_volume: Volume,
    pub distance_to_upper: FixedQ32_32,
    pub distance_to_lower: FixedQ32_32,
    pub regime: BookRegime,
}


#[inline(always)]
pub const fn compute_level_ofi(
    prev_bid: PriceLevel,
    curr_bid: PriceLevel,
    prev_ask: PriceLevel,
    curr_ask: PriceLevel,
) -> i64 {
    let pb_prev = prev_bid.price.0;
    let pb_curr = curr_bid.price.0;
    let vb_prev = prev_bid.volume.0;
    let vb_curr = curr_bid.volume.0;

    // Si le côté vendeur est vide (prix <= 0 lors d'un Fēngbǎn), son prix effectif est +i64::MAX
    let pa_prev = if prev_ask.price.0 <= 0 { i64::MAX } else { prev_ask.price.0 };
    let pa_curr = if curr_ask.price.0 <= 0 { i64::MAX } else { curr_ask.price.0 };
    let va_prev = prev_ask.volume.0;
    let va_curr = curr_ask.volume.0;

    // Conversion primitive bool -> i64 (0 ou 1) 100% compatible const fn et sans branchement CPU
    let bid_ge = (pb_curr >= pb_prev) as i64;
    let bid_le = (pb_curr <= pb_prev) as i64;
    let ask_le = (pa_curr <= pa_prev) as i64;
    let ask_ge = (pa_curr >= pa_prev) as i64;

    let delta_bid = (bid_ge * vb_curr) - (bid_le * vb_prev);
    let delta_ask = (ask_le * va_curr) - (ask_ge * va_prev);

    delta_bid - delta_ask
}