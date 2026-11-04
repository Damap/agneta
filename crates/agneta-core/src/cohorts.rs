use crate::types::{FixedPrice, Side, Volume};
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum InvestorCohort {
    SanhuSmall = 0,
    Medium = 1,
    Large = 2,
    YouziSuperLarge = 3,
}

impl InvestorCohort {
    pub const SUPER_LARGE_YUAN: i64 = 1_000_000;
    pub const LARGE_YUAN: i64 = 200_000;
    pub const MEDIUM_YUAN: i64 = 40_000;
    pub const SUPER_LARGE_SHARES: i64 = 200_000;
    pub const LARGE_SHARES: i64 = 40_000;
    pub const MEDIUM_SHARES: i64 = 10_000;

    #[inline(always)]
    pub const fn classify(price: FixedPrice, volume: Volume) -> Self {
        let shares = volume.0.abs();
        let yuan = price.notional_yuan(Volume(shares));

        if yuan >= Self::SUPER_LARGE_YUAN || shares >= Self::SUPER_LARGE_SHARES {
            Self::YouziSuperLarge
        } else if yuan >= Self::LARGE_YUAN || shares >= Self::LARGE_SHARES {
            Self::Large
        } else if yuan >= Self::MEDIUM_YUAN || shares >= Self::MEDIUM_SHARES {
            Self::Medium
        } else {
            Self::SanhuSmall
        }
    }

    #[inline(always)]
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct CohortFlowTracker {
    pub net_flow_by_cohort: [i64; 4],
    pub gross_volume_by_cohort: [i64; 4],
}

impl CohortFlowTracker {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            net_flow_by_cohort: [0; 4],
            gross_volume_by_cohort: [0; 4],
        }
    }

    #[inline(always)]
    pub fn record_order(&mut self, price: FixedPrice, volume: Volume, side: Side) {
        let cohort = InvestorCohort::classify(price, volume);
        let idx = cohort.index();
        let abs_vol = volume.0.abs();
        self.net_flow_by_cohort[idx] += abs_vol * side.sign();
        self.gross_volume_by_cohort[idx] += abs_vol;
    }

    #[inline(always)]
    pub const fn smart_money_net_flow(&self) -> i64 {
        self.net_flow_by_cohort[InvestorCohort::YouziSuperLarge.index()]
            + self.net_flow_by_cohort[InvestorCohort::Large.index()]
    }
    
    #[inline(always)]
    pub const fn retail_crowd_net_flow(&self) -> i64 {
        self.net_flow_by_cohort[InvestorCohort::SanhuSmall.index()]
    }
    
    #[inline(always)]
    pub const fn institutional_retail_divergence_bps(&self) -> i64 {
        let total_gross = self.gross_volume_by_cohort[0]
            + self.gross_volume_by_cohort[1]
            + self.gross_volume_by_cohort[2]
            + self.gross_volume_by_cohort[3];

        if total_gross <= 0 {
            return 0;
        }
        let diff = self.smart_money_net_flow() - self.retail_crowd_net_flow();
        (diff * 10_000) / total_gross
    }
}