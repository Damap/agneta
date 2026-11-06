use agneta_core::{FixedPrice, Volume};


#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct PriceLevel {
    pub price: FixedPrice,
    pub volume: Volume,
}

impl PriceLevel {
    pub const EMPTY: Self = Self {
        price: FixedPrice::ZERO,
        volume: Volume::ZERO,
    };

    #[inline(always)]
    pub const fn new(price: FixedPrice, volume: Volume) -> Self {
        Self { price, volume }
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.price.0 <= 0 || self.volume.0 <= 0
    }
    
    #[inline(always)]
    pub const fn notional_yuan(self) -> i64 {
        self.price.notional_yuan(self.volume)
    }
}
