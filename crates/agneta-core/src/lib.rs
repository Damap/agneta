#![no_std]
#![forbid(unsafe_code)]

pub mod cohorts;
pub mod events;
pub mod math;
pub mod regime;
pub mod types;

pub use cohorts::{CohortFlowTracker, InvestorCohort};
pub use events::{
    L3OrderAction, L3OrderEvent, L3TradeEvent, SocialEvent, SocialPlatform, SocialSlangFlags,
};
pub use math::{compute_weighted_micro_price, FixedQ32_32};
pub use regime::{BookRegime, MarketBoard, PriceBands, SettlementRule};
pub use types::{FixedPrice, InstrumentId, NanoTime, OrderId, SectorId, Side, Volume};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_q32_32_exp_neg_accuracy() {
        // exp(0) == 1.0
        assert_eq!(FixedQ32_32::ZERO.exp_neg(), FixedQ32_32::ONE);

        // exp(-1.0) ≈ 0.36787944 -> en points de base ≈ 3678 bps
        let exp_minus_one = FixedQ32_32::ONE.exp_neg();
        let bps = exp_minus_one.to_bps();
        assert!((3677..=3680).contains(&bps), "bps got: {bps}");

        // exp(-ln(2)) == 0.5 exactement
        let ln2 = FixedQ32_32::from_raw(2_977_044_472);
        let half = ln2.exp_neg();
        assert_eq!(half.to_bps(), 5000);
    }

    #[test]
    fn test_youzi_vs_sanhu_cohort_classification_and_divergence() {
        let price = FixedPrice::from_yuan(50); // 50.00 RMB par action
        let mut tracker = CohortFlowTracker::new();

        // 1. Un Yóuzī achète 30 000 actions à 50 RMB (Valeur = 1 500 000 RMB >= 1M RMB -> ChaoDaDan)
        let youzi_vol = Volume::from_raw(30_000);
        assert_eq!(
            InvestorCohort::classify(price, youzi_vol),
            InvestorCohort::YouziSuperLarge
        );
        tracker.record_order(price, youzi_vol, Side::Bid);

        // 2. Six petits porteurs (Sǎnhù) vendent chacun 500 actions (Valeur = 25 000 RMB < 40k RMB -> XiaoDan)
        let sanhu_vol = Volume::from_raw(500);
        assert_eq!(
            InvestorCohort::classify(price, sanhu_vol),
            InvestorCohort::SanhuSmall
        );
        for _ in 0..6 {
            tracker.record_order(price, sanhu_vol, Side::Ask);
        }

        // Flux net Smart Money = +30 000, Flux net Retail = -3 000 -> Divergence positive (Accumulation furtive)
        assert_eq!(tracker.smart_money_net_flow(), 30_000);
        assert_eq!(tracker.retail_crowd_net_flow(), -3_000);
        assert_eq!(tracker.institutional_retail_divergence_bps(), 10_000);
    }

    #[test]
    fn test_price_bands_and_magnet_distance_delta() {
        let prev_close = FixedPrice::from_cents(20_00); // 20.00 RMB
        let bands = PriceBands::from_prev_close(prev_close, MarketBoard::MainBoard);

        // ±10% de 20.00 RMB -> [18.00 RMB, 22.00 RMB]
        assert_eq!(bands.lower_limit, FixedPrice::from_cents(18_00));
        assert_eq!(bands.upper_limit, FixedPrice::from_cents(22_00));

        // À 21.00 RMB (à mi-chemin entre 20.00 et la limite haute 22.00), δ_t = 0.5 (5000 bps)
        let mid_rally = FixedPrice::from_cents(21_00);
        let delta = bands.normalized_distance_to_upper(mid_rally);
        assert_eq!(delta.to_bps(), 5000);

        // À 21.50 RMB (+7.5% > 7.0%), le titre franchit le seuil de publication de la Lónghǔbǎng (龙虎榜)
        let lhb_price = FixedPrice::from_cents(21_50);
        assert!(bands.is_above_longhubang_threshold(lhb_price));
    }
}