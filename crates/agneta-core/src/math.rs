//! Arithmétique en virgule fixe Q32.32 et fonctions transcendantales sans flottants.

use core::ops::{Add, AddAssign, Sub, SubAssign};
use crate::types::{FixedPrice, Volume};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedQ32_32(pub i64);

impl FixedQ32_32 {
    pub const FRAC_BITS: u32 = 32;
    pub const ONE: Self = Self(1_i64 << Self::FRAC_BITS);
    pub const HALF: Self = Self(1_i64 << (Self::FRAC_BITS - 1));
    pub const ZERO: Self = Self(0);

    #[inline(always)]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    #[inline(always)]
    pub const fn from_int(val: i32) -> Self {
        Self((val as i64) << Self::FRAC_BITS)
    }

    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_bps(bps: i64) -> Self {
        let num = (bps as i128) << Self::FRAC_BITS;
        let rounded = if num >= 0 { num + 5_000 } else { num - 5_000 };
        Self((rounded / 10_000) as i64)
    }

    #[inline(always)]
    pub const fn to_int(self) -> i64 {
        self.0 >> Self::FRAC_BITS
    }

    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn to_bps(self) -> i64 {
        let prod = (self.0 as i128) * 10_000;
        let half = 1_i128 << (Self::FRAC_BITS - 1);
        let rounded = if prod >= 0 { prod + half } else { prod - half };
        (rounded >> Self::FRAC_BITS) as i64
    }

    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn mul(self, rhs: Self) -> Self {
        let prod = (self.0 as i128) * (rhs.0 as i128);
        let half = 1_i128 << (Self::FRAC_BITS - 1);
        let rounded = if prod >= 0 { prod + half } else { prod - half };
        Self((rounded >> Self::FRAC_BITS) as i64)
    }
    
    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn checked_div(self, rhs: Self) -> Self {
        if rhs.0 == 0 {
            return Self::ZERO;
        }
        let num = (self.0 as i128) << Self::FRAC_BITS;
        let den = rhs.0 as i128;
        let half_den = den.abs() >> 1;
        let rounded = if (num >= 0) == (den >= 0) {
            num + half_den
        } else {
            num - half_den
        };
        Self((rounded / den) as i64)
    }
    
    #[inline(always)]
    pub const fn exp_neg(self) -> Self {
        if self.0 <= 0 {
            return Self::ONE;
        }
        if self.0 >= (22_i64 << Self::FRAC_BITS) {
            return Self::ZERO;
        }

        // Constante ln(2) en Q32.32 : round(0.6931471805599453 * 2^32) = 2_977_044_472
        const LN2_Q32: i64 = 2_977_044_472;

        let k = self.0 / LN2_Q32;
        let r = Self(self.0 - k * LN2_Q32);

        let r2 = r.mul(r);
        let six_r = Self(r.0 * 6);
        let twelve = Self::from_int(12);

        let num = Self(twelve.0 - six_r.0 + r2.0);
        let den = Self(twelve.0 + six_r.0 + r2.0);
        let exp_r = num.checked_div(den);

        Self(exp_r.0 >> k)
    }
}

impl Add for FixedQ32_32 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for FixedQ32_32 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for FixedQ32_32 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for FixedQ32_32 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
pub const fn compute_weighted_micro_price(
    bid_price: FixedPrice,
    bid_vol: Volume,
    ask_price: FixedPrice,
    ask_vol: Volume,
) -> FixedPrice {
    let pb = bid_price.0 as i128;
    let vb = bid_vol.0 as i128;
    let pa = ask_price.0 as i128;
    let va = ask_vol.0 as i128;

    let total_vol = va + vb;
    if total_vol <= 0 {
        return FixedPrice::ZERO;
    }

    let numerator = (va * pb) + (vb * pa) + (total_vol >> 1);
    FixedPrice((numerator / total_vol) as i64)
}