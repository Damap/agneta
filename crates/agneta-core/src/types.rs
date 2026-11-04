use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NanoTime(pub u64);

impl NanoTime {
    pub const NANOS_PER_MILLI: u64 = 1_000_000;
    pub const NANOS_PER_SEC: u64 = 1_000_000_000;

    #[inline(always)]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    #[inline(always)]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis.saturating_mul(Self::NANOS_PER_MILLI))
    }

    #[inline(always)]
    pub const fn delta_ns(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedPrice(pub i64);

impl FixedPrice {
    pub const SCALE: i64 = 10_000;
    pub const ASHARE_TICK: i64 = 100;
    pub const ZERO: Self = Self(0);

    #[inline(always)]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    #[inline(always)]
    pub const fn from_cents(cents: i64) -> Self {
        Self(cents * Self::ASHARE_TICK)
    }

    #[inline(always)]
    pub const fn from_yuan(yuan: i64) -> Self {
        Self(yuan * Self::SCALE)
    }

    #[inline(always)]
    pub const fn raw(self) -> i64 {
        self.0
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.0 <= 0
    }

    #[inline(always)]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn notional_yuan(self, volume: Volume) -> i64 {
        let prod = (self.0 as i128) * (volume.0 as i128);
        (prod / (Self::SCALE as i128)) as i64
    }
}

impl Add for FixedPrice {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for FixedPrice {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl fmt::Display for FixedPrice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        let scale_u64 = Self::SCALE.unsigned_abs();
        let integer = abs / scale_u64;
        let fraction = abs % scale_u64;
        write!(f, "{}{}.{:04}", sign, integer, fraction)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Volume(pub i64);

impl Volume {
    pub const ZERO: Self = Self(0);

    #[inline(always)]
    pub const fn from_raw(v: i64) -> Self {
        Self(v)
    }

    #[inline(always)]
    pub const fn raw(self) -> i64 {
        self.0
    }
}

impl Add for Volume {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Volume {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Volume {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Volume {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

#[repr(i8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Bid = 1,
    Ask = -1,
}

impl Side {
    #[inline(always)]
    pub const fn sign(self) -> i64 {
        self as i64
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderId(pub u64);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectorId(pub u32);


#[repr(C, align(16))]
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstrumentId([u8; 16]);

impl InstrumentId {
    #[inline(always)]
    pub const fn from_bytes(bytes: &[u8]) -> Self {
        let mut buf = [0u8; 16];
        let len = if bytes.len() < 16 { bytes.len() } else { 16 };
        let mut i = 0;
        while i < len {
            buf[i] = bytes[i];
            i += 1;
        }
        Self(buf)
    }
    
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(16);
        &self.0[..len]
    }
}

impl fmt::Debug for InstrumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = core::str::from_utf8(self.as_bytes()).unwrap_or("<invalid-ascii>");
        write!(f, "InstrumentId({})", s)
    }
}