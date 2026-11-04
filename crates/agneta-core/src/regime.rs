use crate::math::FixedQ32_32;
use crate::types::FixedPrice;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MarketBoard {
    SpecialTreatment = 0,
    MainBoard = 1,
    ChiNextOrStar = 2,
    BeijingSE = 3,
}

impl MarketBoard {
    #[inline(always)]
    pub const fn limit_bps(self) -> i64 {
        match self {
            Self::SpecialTreatment => 500,
            Self::MainBoard => 1_000,
            Self::ChiNextOrStar => 2_000,
            Self::BeijingSE => 3_000,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SettlementRule {
    TPlusOne = 1,
    TPlusZero = 0,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PriceBands {
    pub prev_close: FixedPrice,
    pub lower_limit: FixedPrice,
    pub upper_limit: FixedPrice,
}


impl PriceBands {
    #[inline(always)]
    pub const fn from_prev_close(prev_close: FixedPrice, board: MarketBoard) -> Self {
        let limit_bps = board.limit_bps();
        let prev_cents = prev_close.0 / FixedPrice::ASHARE_TICK;

        let upper_cents = (prev_cents * (10_000 + limit_bps) + 5_000) / 10_000;
        let lower_cents = (prev_cents * (10_000 - limit_bps) + 5_000) / 10_000;

        Self {
            prev_close,
            lower_limit: FixedPrice::from_cents(lower_cents),
            upper_limit: FixedPrice::from_cents(upper_cents),
        }
    }

    #[inline(always)]
    pub const fn normalized_distance_to_upper(self, current_price: FixedPrice) -> FixedQ32_32 {
        let max_span = self.upper_limit.0 - self.prev_close.0;
        if max_span <= 0 {
            return FixedQ32_32::ONE;
        }
        let remaining = self.upper_limit.0 - current_price.0;
        if remaining <= 0 {
            return FixedQ32_32::ZERO;
        }
        if remaining >= max_span {
            return FixedQ32_32::ONE;
        }
        let num = FixedQ32_32(remaining);
        let den = FixedQ32_32(max_span);
        num.checked_div(den)
    }

    #[inline(always)]
    pub const fn is_above_longhubang_threshold(self, close_price: FixedPrice) -> bool {
        if self.prev_close.0 <= 0 {
            return false;
        }
        let diff = (close_price.0 - self.prev_close.0).abs();
        // Déviation >= 7.00% (700 bps)
        (diff * 10_000) >= (self.prev_close.0 * 700)
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BookRegime {
    Normal = 0,
    MagnetZoneUp = 1,
    LimitUpLocked = 2,
    MagnetZoneDown = 3,
    LimitDownLocked = 4,
    HaltedOrAuction = 5,
}