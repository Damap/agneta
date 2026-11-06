use agneta_core::{BookRegime, FixedPrice, NanoTime, Volume};
use crate::metrics::TickMetrics;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MicrostructureBar {
    pub open_time: NanoTime,
    pub close_time: NanoTime,
    pub open_micro: FixedPrice,
    pub high_micro: FixedPrice,
    pub low_micro: FixedPrice,
    pub close_micro: FixedPrice,
    pub bar_volume: Volume,
    pub cumulative_ofi_l1: i64,
    pub cumulative_ofi_deep: i64,
    pub max_locked_limit_volume: Volume,
    pub close_regime: BookRegime,
}

#[derive(Clone, Debug)]
pub struct BarAggregator {
    pub volume_threshold: Volume,
    pub ofi_imbalance_threshold: i64,
    last_book_cum_vol: Volume,
    current: Option<MicrostructureBar>,
}

impl BarAggregator {
    #[inline(always)]
    pub const fn new(volume_threshold: Volume, ofi_imbalance_threshold: i64) -> Self {
        Self {
            volume_threshold,
            ofi_imbalance_threshold,
            last_book_cum_vol: Volume::ZERO,
            current: None,
        }
    }
    
    #[inline(always)]
    pub fn on_tick(
        &mut self,
        metrics: &TickMetrics,
        book_cumulative_volume: Volume,
    ) -> Option<MicrostructureBar> {
        let delta_vol = if self.last_book_cum_vol.0 == 0 && self.current.is_none() {
            Volume::ZERO
        } else {
            let diff = book_cumulative_volume.0 - self.last_book_cum_vol.0;
            Volume(if diff > 0 { diff } else { 0 })
        };
        self.last_book_cum_vol = book_cumulative_volume;

        let p = metrics.micro_price;
        let bar = match self.current.take() {
            None => MicrostructureBar {
                open_time: metrics.timestamp,
                close_time: metrics.timestamp,
                open_micro: p,
                high_micro: p,
                low_micro: p,
                close_micro: p,
                bar_volume: delta_vol,
                cumulative_ofi_l1: metrics.ofi_l1,
                cumulative_ofi_deep: metrics.ofi_deep,
                max_locked_limit_volume: metrics.locked_limit_volume,
                close_regime: metrics.regime,
            },
            Some(mut b) => {
                b.close_time = metrics.timestamp;
                if p.0 > b.high_micro.0 {
                    b.high_micro = p;
                }
                if p.0 < b.low_micro.0 || b.low_micro.0 == 0 {
                    b.low_micro = p;
                }
                b.close_micro = p;
                b.bar_volume += delta_vol;
                b.cumulative_ofi_l1 += metrics.ofi_l1;
                b.cumulative_ofi_deep += metrics.ofi_deep;
                if metrics.locked_limit_volume.0 > b.max_locked_limit_volume.0 {
                    b.max_locked_limit_volume = metrics.locked_limit_volume;
                }
                b.close_regime = metrics.regime;
                b
            }
        };

        let vol_reached =
            self.volume_threshold.0 > 0 && bar.bar_volume.0 >= self.volume_threshold.0;
        let ofi_reached = self.ofi_imbalance_threshold > 0
            && bar.cumulative_ofi_l1.abs() >= self.ofi_imbalance_threshold;

        if vol_reached || ofi_reached {
            Some(bar)
        } else {
            self.current = Some(bar);
            None
        }
    }
}