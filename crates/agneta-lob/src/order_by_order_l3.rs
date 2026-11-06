//! Analyseur de flux L3 Order-by-Order (逐笔委托 / 逐笔成交) et détection du Spoofing sur Fēngbǎn.

use agneta_core::{
    CohortFlowTracker, InstrumentId, L3OrderAction, L3OrderEvent, L3TradeEvent, NanoTime,
    PriceBands, Side,
};
use crate::parent_order::ParentOrderTable;

#[derive(Clone, Debug)]
pub struct OrderByOrderAnalyzerL3<const TABLE_CAP: usize = 1024> {
    pub instrument: InstrumentId,
    pub bands: PriceBands,
    pub last_timestamp: NanoTime,
    pub cohort_tracker: CohortFlowTracker,
    pub parent_table: ParentOrderTable<TABLE_CAP>,
    pub total_bid_added_volume: i64,
    pub total_bid_canceled_volume: i64,
    pub upper_limit_bid_added_volume: i64,
    pub upper_limit_bid_canceled_volume: i64,
}

impl<const TABLE_CAP: usize> OrderByOrderAnalyzerL3<TABLE_CAP> {
    #[inline(always)]
    pub const fn new(instrument: InstrumentId, bands: PriceBands) -> Self {
        Self {
            instrument,
            bands,
            last_timestamp: NanoTime(0),
            cohort_tracker: CohortFlowTracker::new(),
            parent_table: ParentOrderTable::new(),
            total_bid_added_volume: 0,
            total_bid_canceled_volume: 0,
            upper_limit_bid_added_volume: 0,
            upper_limit_bid_canceled_volume: 0,
        }
    }
    
    #[inline(always)]
    pub fn on_order(&mut self, event: &L3OrderEvent) {
        self.last_timestamp = event.timestamp;
        let vol = event.volume.0.abs();
        let is_at_upper_limit = event.price.0 >= self.bands.upper_limit.0;

        if event.side == Side::Bid {
            match event.action {
                L3OrderAction::AddLimit | L3OrderAction::Market => {
                    self.total_bid_added_volume += vol;
                    if is_at_upper_limit {
                        self.upper_limit_bid_added_volume += vol;
                    }
                }
                L3OrderAction::Cancel => {
                    self.total_bid_canceled_volume += vol;
                    if is_at_upper_limit {
                        self.upper_limit_bid_canceled_volume += vol;
                    }
                }
            }
        }
    }
    
    #[inline(always)]
    pub fn on_trade(&mut self, trade: &L3TradeEvent) {
        self.last_timestamp = trade.timestamp;

        // Identification du numéro d'ordre parent du côté de l'initiateur agressif
        let active_parent_id = match trade.aggressor_side {
            Side::Bid => trade.buy_order_id,
            Side::Ask => trade.sell_order_id,
        };

        self.parent_table.observe_fill_and_update_cohort(
            active_parent_id,
            trade.price,
            trade.volume,
            trade.aggressor_side,
            &mut self.cohort_tracker,
        );
    }
    
    #[inline(always)]
    pub const fn bid_cancel_ratio_bps(&self) -> i64 {
        if self.total_bid_added_volume <= 0 {
            return 0;
        }
        (self.total_bid_canceled_volume * 10_000) / self.total_bid_added_volume
    }
    
    #[inline(always)]
    pub const fn upper_limit_spoofing_ratio_bps(&self) -> i64 {
        if self.upper_limit_bid_added_volume <= 0 {
            return 0;
        }
        (self.upper_limit_bid_canceled_volume * 10_000) / self.upper_limit_bid_added_volume
    }
}