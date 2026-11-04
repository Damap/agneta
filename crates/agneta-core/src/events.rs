use crate::types::{FixedPrice, InstrumentId, NanoTime, OrderId, SectorId, Side, Volume};

/// Social platform source of the textual event.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SocialPlatform {
    EastMoneyGuba = 0,
    Taoguba = 1,
    Xueqiu = 2,
    Cailianshe = 3,
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SocialSlangFlags(pub u8);

impl SocialSlangFlags {
    pub const NONE: Self = Self(0);
    pub const DABAN: u8 = 1 << 0;
    pub const NUCLEAR_PANIC: u8 = 1 << 1;
    pub const SECTOR_LEADER: u8 = 1 << 2;
    pub const BULL_TRAP_LOSS: u8 = 1 << 3;
    pub const DISTRIBUTION_ALERT: u8 = 1 << 4;
    pub const LONG_HU_BANG: u8 = 1 << 5;

    #[inline(always)]
    pub const fn contains(self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }
}

#[repr(C, align(32))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SocialEvent {
    pub timestamp: NanoTime,
    pub instrument: InstrumentId,
    pub sector: SectorId,
    pub sentiment_bps: i32,
    pub author_weight_bps: u16,
    pub platform: SocialPlatform,
    pub slang_flags: SocialSlangFlags,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum L3OrderAction {
    AddLimit = 0,
    Cancel = 1,
    Market = 2,
}
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct L3OrderEvent {
    pub timestamp: NanoTime,
    pub order_id: OrderId,
    pub price: FixedPrice,
    pub volume: Volume,
    pub side: Side,
    pub action: L3OrderAction,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct L3TradeEvent {
    pub timestamp: NanoTime,
    pub buy_order_id: OrderId,
    pub sell_order_id: OrderId,
    pub price: FixedPrice,
    pub volume: Volume,
    pub aggressor_side: Side,
}