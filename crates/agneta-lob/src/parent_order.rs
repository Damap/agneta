use agneta_core::{CohortFlowTracker, FixedPrice, InvestorCohort, OrderId, Side, Volume};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ParentOrderEntry {
    pub order_id: OrderId,
    pub cumulative_volume: Volume,
    pub reference_price: FixedPrice,
    pub current_cohort_idx: u8,
}

#[derive(Clone, Debug)]
pub struct ParentOrderTable<const CAPACITY: usize = 1024> {
    entries: [ParentOrderEntry; CAPACITY],
}

impl<const CAPACITY: usize> Default for ParentOrderTable<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize> ParentOrderTable<CAPACITY> {
    const EMPTY_ENTRY: ParentOrderEntry = ParentOrderEntry {
        order_id: OrderId(0),
        cumulative_volume: Volume::ZERO,
        reference_price: FixedPrice::ZERO,
        current_cohort_idx: 0,
    };
    
    #[inline(always)]
    pub const fn new() -> Self {
        assert!(CAPACITY.is_power_of_two(), "CAPACITY must be a power of two");
        Self {
            entries: [Self::EMPTY_ENTRY; CAPACITY],
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.entries = [Self::EMPTY_ENTRY; CAPACITY];
    }

    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    const fn hash_index(order_id: OrderId) -> usize {
        let h = order_id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        (h as usize) & (CAPACITY - 1)
    }
    
    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    pub fn observe_fill_and_update_cohort(
        &mut self,
        order_id: OrderId,
        fill_price: FixedPrice,
        fill_volume: Volume,
        side: Side,
        tracker: &mut CohortFlowTracker,
    ) -> InvestorCohort {
        let abs_fill = fill_volume.0.abs();
        if order_id.0 == 0 || abs_fill == 0 {
            let cohort = InvestorCohort::classify(fill_price, fill_volume);
            tracker.record_order(fill_price, fill_volume, side);
            return cohort;
        }

        let mask = CAPACITY - 1;
        let start_idx = Self::hash_index(order_id);
        let mut step = 0;

        // Sondage linéaire borné à 8 sauts pour garantir une latence O(1) constante
        while step < 8 {
            let idx = (start_idx + step) & mask;
            let entry = &mut self.entries[idx];

            // Cas 1 : Nouvel ordre parent jamais vu
            if entry.order_id.0 == 0 {
                let cohort = InvestorCohort::classify(fill_price, Volume(abs_fill));
                let c_idx = cohort.index();
                *entry = ParentOrderEntry {
                    order_id,
                    cumulative_volume: Volume(abs_fill),
                    reference_price: fill_price,
                    current_cohort_idx: c_idx as u8,
                };
                tracker.net_flow_by_cohort[c_idx] += abs_fill * side.sign();
                tracker.gross_volume_by_cohort[c_idx] += abs_fill;
                return cohort;
            }

            // Cas 2 : Ordre parent existant -> cumul du volume et reclassement éventuel
            if entry.order_id == order_id {
                let prev_vol = entry.cumulative_volume.0;
                let prev_cohort_idx = entry.current_cohort_idx as usize;
                let new_vol = prev_vol + abs_fill;
                let new_cohort = InvestorCohort::classify(fill_price, Volume(new_vol));
                let new_cohort_idx = new_cohort.index();

                if new_cohort_idx == prev_cohort_idx {
                    // Même cohorte : on ajoute simplement le nouveau fragment
                    tracker.net_flow_by_cohort[new_cohort_idx] += abs_fill * side.sign();
                    tracker.gross_volume_by_cohort[new_cohort_idx] += abs_fill;
                } else {
                    // Promotion de cohorte (ex: Sǎnhù -> Yóuzī) : transfert rétroactif du volume antérieur !
                    tracker.net_flow_by_cohort[prev_cohort_idx] -= prev_vol * side.sign();
                    tracker.gross_volume_by_cohort[prev_cohort_idx] -= prev_vol;

                    tracker.net_flow_by_cohort[new_cohort_idx] += new_vol * side.sign();
                    tracker.gross_volume_by_cohort[new_cohort_idx] += new_vol;
                    entry.current_cohort_idx = new_cohort_idx as u8;
                }

                entry.cumulative_volume = Volume(new_vol);
                entry.reference_price = fill_price;
                return new_cohort;
            }

            step += 1;
        }

        // Si les 8 slots sont occupés (collision rare), remplacement direct du slot initial
        let cohort = InvestorCohort::classify(fill_price, Volume(abs_fill));
        let c_idx = cohort.index();
        self.entries[start_idx] = ParentOrderEntry {
            order_id,
            cumulative_volume: Volume(abs_fill),
            reference_price: fill_price,
            current_cohort_idx: c_idx as u8,
        };
        tracker.net_flow_by_cohort[c_idx] += abs_fill * side.sign();
        tracker.gross_volume_by_cohort[c_idx] += abs_fill;
        cohort
    }
}