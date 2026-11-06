use agneta_core::{
    compute_weighted_micro_price, BookRegime, FixedPrice, FixedQ32_32, InstrumentId, NanoTime,
    PriceBands, Volume,
};
use crate::level::PriceLevel;
use crate::metrics::{compute_level_ofi, TickMetrics};

#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BookSnapshot<const DEPTH: usize = 5> {
    pub timestamp: NanoTime,
    pub bids: [PriceLevel; DEPTH],
    pub asks: [PriceLevel; DEPTH],
    pub cumulative_volume: Volume,
    pub cumulative_turnover_yuan: i64,
}

#[repr(C, align(64))]
#[derive(Clone, Debug)]
pub struct LimitOrderBookL2<const DEPTH: usize = 5> {
    pub instrument: InstrumentId,
    pub bands: PriceBands,
    pub last_update: NanoTime,
    pub bids: [PriceLevel; DEPTH],
    pub asks: [PriceLevel; DEPTH],
    pub cumulative_volume: Volume,
    initialized: bool,
}

impl<const DEPTH: usize> LimitOrderBookL2<DEPTH> {
    pub const MAGNET_ZONE_THRESHOLD: FixedQ32_32 = FixedQ32_32::from_bps(2_000);

    #[inline(always)]
    pub const fn new(instrument: InstrumentId, bands: PriceBands) -> Self {
        Self {
            instrument,
            bands,
            last_update: NanoTime(0),
            bids: [PriceLevel::EMPTY; DEPTH],
            asks: [PriceLevel::EMPTY; DEPTH],
            cumulative_volume: Volume::ZERO,
            initialized: false,
        }
    }

    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn apply_snapshot(&mut self, snap: &BookSnapshot<DEPTH>) -> TickMetrics {
        let delta_ns = if self.initialized {
            snap.timestamp.delta_ns(self.last_update)
        } else {
            0
        };

        let (ofi_l1, ofi_deep) = if self.initialized {
            let l1 = compute_level_ofi(self.bids[0], snap.bids[0], self.asks[0], snap.asks[0]);
            let mut deep = 0_i64;
            let mut lvl = 0;
            while lvl < DEPTH {
                let weight = (DEPTH - lvl) as i64;
                let lvl_ofi = compute_level_ofi(
                    self.bids[lvl],
                    snap.bids[lvl],
                    self.asks[lvl],
                    snap.asks[lvl],
                );
                deep += lvl_ofi * weight;
                lvl += 1;
            }
            (l1, deep)
        } else {
            (0, 0)
        };

        self.bids = snap.bids;
        self.asks = snap.asks;
        self.last_update = snap.timestamp;
        self.cumulative_volume = snap.cumulative_volume;
        self.initialized = true;

        let (regime, micro_price, distance_to_upper, distance_to_lower) =
            self.evaluate_regime_and_price();
        let spread = self.compute_spread(regime);
        let book_imbalance_bps = self.compute_l1_imbalance_bps();
        let locked_limit_volume = self.locked_limit_volume(regime);

        TickMetrics {
            timestamp: snap.timestamp,
            delta_ns,
            micro_price,
            spread,
            ofi_l1,
            ofi_deep,
            book_imbalance_bps,
            locked_limit_volume,
            distance_to_upper,
            distance_to_lower,
            regime,
        }
    }

    #[inline(always)]
    pub const fn evaluate_regime_and_price(
        &self,
    ) -> (BookRegime, FixedPrice, FixedQ32_32, FixedQ32_32) {
        let b1 = self.bids[0];
        let a1 = self.asks[0];

        if b1.volume.0 <= 0 && a1.volume.0 <= 0 {
            return (
                BookRegime::HaltedOrAuction,
                FixedPrice::ZERO,
                FixedQ32_32::ONE,
                FixedQ32_32::ONE,
            );
        }

        // Case 1 : 涨停封板 (Limit-Up Locked) -> Ask L1 empty and Bid L1 at max level
        if a1.volume.0 <= 0 && b1.volume.0 > 0 && b1.price.0 >= self.bands.upper_limit.0 {
            let p = b1.price;
            return (
                BookRegime::LimitUpLocked,
                p,
                FixedQ32_32::ZERO,
                self.bands.normalized_distance_to_lower(p),
            );
        }

        // Case 2 : 跌停封板 (Limit-Down Locked) -> Bid L1 empty and Ask L1 au plancher
        if b1.volume.0 <= 0 && a1.volume.0 > 0 && a1.price.0 <= self.bands.lower_limit.0 {
            let p = a1.price;
            return (
                BookRegime::LimitDownLocked,
                p,
                self.bands.normalized_distance_to_upper(p),
                FixedQ32_32::ZERO,
            );
        }

        // Case 3 : Cotation bilatérale -> calcul du Micro-Price pondéré en 128 bits
        let micro_price = compute_weighted_micro_price(b1.price, b1.volume, a1.price, a1.volume);
        let dist_up = self.bands.normalized_distance_to_upper(micro_price);
        let dist_down = self.bands.normalized_distance_to_lower(micro_price);

        let regime = if dist_up.0 <= Self::MAGNET_ZONE_THRESHOLD.0 {
            BookRegime::MagnetZoneUp
        } else if dist_down.0 <= Self::MAGNET_ZONE_THRESHOLD.0 {
            BookRegime::MagnetZoneDown
        } else {
            BookRegime::Normal
        };

        (regime, micro_price, dist_up, dist_down)
    }

    #[inline(always)]
    pub const fn compute_spread(&self, regime: BookRegime) -> FixedPrice {
        match regime {
            BookRegime::Normal | BookRegime::MagnetZoneUp | BookRegime::MagnetZoneDown => {
                FixedPrice(self.asks[0].price.0 - self.bids[0].price.0)
            }
            _ => FixedPrice::ZERO,
        }
    }

    #[inline(always)]
    pub const fn compute_l1_imbalance_bps(&self) -> i64 {
        let vb = self.bids[0].volume.0;
        let va = self.asks[0].volume.0;
        let total = vb + va;
        if total <= 0 {
            return 0;
        }
        ((vb - va) * 10_000) / total
    }

    #[inline(always)]
    pub const fn locked_limit_volume(&self, regime: BookRegime) -> Volume {
        match regime {
            BookRegime::LimitUpLocked => self.bids[0].volume,
            BookRegime::LimitDownLocked => self.asks[0].volume,
            _ => Volume::ZERO,
        }
    }
}