#![no_std]
#![forbid(unsafe_code)]

pub mod bars;
pub mod level;
pub mod metrics;
pub mod order_by_order_l3;
pub mod parent_order;
pub mod snapshot_l2;

pub use bars::{BarAggregator, MicrostructureBar};
pub use level::PriceLevel;
pub use metrics::{compute_level_ofi, TickMetrics};
pub use order_by_order_l3::OrderByOrderAnalyzerL3;
pub use parent_order::{ParentOrderEntry, ParentOrderTable};
pub use snapshot_l2::{BookSnapshot, LimitOrderBookL2};

#[cfg(test)]
mod tests {
    use super::*;
    use agneta_core::{
        BookRegime, FixedPrice, InstrumentId, InvestorCohort, L3OrderAction, L3OrderEvent,
        L3TradeEvent, MarketBoard, NanoTime, OrderId, PriceBands, Side, Volume,
    };

    #[test]
    fn test_l2_snapshot_magnet_zone_and_fengban_locking() {
        let inst = InstrumentId::from_bytes(b"600519.SH");
        // Clôture veille = 10.00 RMB -> Limites ±10% = [9.00 RMB, 11.00 RMB]
        let bands = PriceBands::from_prev_close(FixedPrice::from_cents(10_00), MarketBoard::MainBoard);
        let mut lob = LimitOrderBookL2::<5>::new(inst, bands);

        // Tick 1 : Prix autour de 10.85 RMB -> δ_t = (11.00 - 10.85) / 1.00 = 0.15 <= 0.20 -> MagnetZoneUp !
        let mut snap1 = BookSnapshot {
            timestamp: NanoTime::from_millis(1_000),
            bids: [PriceLevel::EMPTY; 5],
            asks: [PriceLevel::EMPTY; 5],
            cumulative_volume: Volume::from_raw(10_000),
            cumulative_turnover_yuan: 108_500,
        };
        snap1.bids[0] = PriceLevel::new(FixedPrice::from_cents(10_84), Volume::from_raw(500));
        snap1.asks[0] = PriceLevel::new(FixedPrice::from_cents(10_86), Volume::from_raw(500));

        let m1 = lob.apply_snapshot(&snap1);
        assert_eq!(m1.micro_price, FixedPrice::from_cents(10_85));
        assert_eq!(m1.regime, BookRegime::MagnetZoneUp);
        assert_eq!(m1.distance_to_upper.to_bps(), 1500);

        // Tick 2 : Verrouillage en Dǎbǎn / Fēngbǎn à 11.00 RMB avec 80 000 titres en Fēngdān
        let mut snap2 = snap1;
        snap2.timestamp = NanoTime::from_millis(1_500);
        snap2.bids[0] = PriceLevel::new(FixedPrice::from_cents(11_00), Volume::from_raw(80_000));
        snap2.asks[0] = PriceLevel::EMPTY;
        snap2.cumulative_volume = Volume::from_raw(50_000);

        let m2 = lob.apply_snapshot(&snap2);
        assert_eq!(m2.regime, BookRegime::LimitUpLocked);
        assert_eq!(m2.locked_limit_volume, Volume::from_raw(80_000));
        assert_eq!(m2.distance_to_upper.to_bps(), 0);
        assert!(m2.ofi_l1 > 0);
    }

    #[test]
    fn test_l3_parent_order_reaggregation_unmasks_youzi_split_orders() {
        let inst = InstrumentId::from_bytes(b"300750.SZ");
        let bands = PriceBands::from_prev_close(FixedPrice::from_yuan(50), MarketBoard::ChiNextOrStar);
        let mut l3 = OrderByOrderAnalyzerL3::<1024>::new(inst, bands);

        // Un Yóuzī place un ordre d'achat massif (BuyNo = 999_888) à 50.00 RMB.
        // Il est exécuté en 40 petits fragments de 600 actions contre des vendeurs Sǎnhù (SellNo différents).
        // Chaque fragment isolé ne vaut que 600 * 50 = 30 000 RMB (< 40 000 RMB -> ressemblerait à un Sǎnhù !).
        // Mais cumulés (40 * 30 000 = 1 200 000 RMB >= 1M RMB), la table L3 doit reclasser la totalité en YouziSuperLarge !
        let parent_buy_id = OrderId(999_888);
        let price = FixedPrice::from_yuan(50);

        for i in 1..=40_u64 {
            let trade = L3TradeEvent {
                timestamp: NanoTime::from_millis(i * 10),
                buy_order_id: parent_buy_id,
                sell_order_id: OrderId(10_000 + i),
                price,
                volume: Volume::from_raw(600),
                aggressor_side: Side::Bid,
            };
            l3.on_trade(&trade);
        }

        // Vérification : 100% des 24 000 actions ont été transférées rétroactivement dans YouziSuperLarge (index 3)
        // et le compteur SanhuSmall (index 0) est retombé exactement à 0 !
        assert_eq!(l3.cohort_tracker.retail_crowd_net_flow(), 0);
        assert_eq!(
            l3.cohort_tracker.net_flow_by_cohort[InvestorCohort::YouziSuperLarge.index()],
            24_000
        );
        assert_eq!(l3.cohort_tracker.smart_money_net_flow(), 24_000);
    }

    #[test]
    fn test_l3_upper_limit_spoofing_detection() {
        let inst = InstrumentId::from_bytes(b"000001.SZ");
        let bands = PriceBands::from_prev_close(FixedPrice::from_cents(10_00), MarketBoard::MainBoard);
        let mut l3 = OrderByOrderAnalyzerL3::<512>::new(inst, bands);

        // Ajout de 100 000 actions sur le Fēngbǎn (11.00 RMB) pour attirer les Sǎnhù
        l3.on_order(&L3OrderEvent {
            timestamp: NanoTime::from_millis(100),
            order_id: OrderId(1),
            price: bands.upper_limit,
            volume: Volume::from_raw(100_000),
            side: Side::Bid,
            action: L3OrderAction::AddLimit,
        });

        // Retrait (撤单) de 75 000 actions dès que les Sǎnhù entrent dans la file
        l3.on_order(&L3OrderEvent {
            timestamp: NanoTime::from_millis(250),
            order_id: OrderId(1),
            price: bands.upper_limit,
            volume: Volume::from_raw(75_000),
            side: Side::Bid,
            action: L3OrderAction::Cancel,
        });

        // Taux de Spoofing sur la limite haute = 75% (7500 bps)
        assert_eq!(l3.upper_limit_spoofing_ratio_bps(), 7_500);
    }
}