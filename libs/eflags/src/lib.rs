#![feature(associated_type_defaults)]
#![no_std]

use core::clone::Clone;
use core::cmp::{Eq, PartialEq};
use core::iter::Iterator;
use core::marker::Copy;
use core::{
    fmt::Debug,
    ops::{BitAnd, BitOr, BitXor, Not},
};

pub use eflags_macros::eflags;

pub trait UInt:
    Copy
    + BitXor<Output = Self>
    + BitAnd<Output = Self>
    + BitOr<Output = Self>
    + Not<Output = Self>
    + PartialEq
    + Eq
    + Debug
    + Default
{
}

impl UInt for u8 {}
impl UInt for u16 {}
impl UInt for u32 {}
impl UInt for u64 {}

pub trait Flag: 'static + Copy + PartialEq + Eq {
    type Value: UInt;

    fn from_raw(value: Self::Value) -> Self;

    fn name(&self) -> &'static str;

    fn value(&self) -> Self::Value;

    fn values() -> &'static [Self];
}

pub trait FlagOrFlagSet<F: Flag>: Copy {
    fn value(&self) -> F::Value;
}

impl<F: Flag> FlagOrFlagSet<F> for F {
    fn value(&self) -> F::Value {
        Flag::value(self)
    }
}

impl<F: Flag> FlagOrFlagSet<F> for FlagSet<F> {
    fn value(&self) -> F::Value {
        self.value
    }
}

pub struct FlagSet<F: Flag> {
    value: F::Value,
}

impl<F: Flag> Default for FlagSet<F> {
    fn default() -> Self {
        FlagSet::empty()
    }
}

impl<F: Flag> FlagSet<F> {
    pub fn empty() -> Self {
        FlagSet {
            value: F::Value::default(),
        }
    }

    pub fn from<const N: usize>(flags: [F; N]) -> Self {
        let mut set = Self::default();
        for flag in flags.iter() {
            set |= *flag;
        }
        set
    }

    pub fn from_raw(value: F::Value) -> Self {
        FlagSet { value: value }
    }

    pub const fn value(&self) -> F::Value {
        self.value
    }

    pub fn contains(&self, flags: impl FlagOrFlagSet<F>) -> bool {
        (self.value & flags.value()) == flags.value()
    }
}

impl<F: Flag> core::ops::Not for FlagSet<F> {
    type Output = FlagSet<F>;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Self { value: !self.value }
    }
}

// FSet & {F, Fset}
impl<F: Flag> core::ops::BitAnd<F> for FlagSet<F> {
    type Output = FlagSet<F>;

    #[inline(always)]
    fn bitand(self, rhs: F) -> Self::Output {
        Self {
            value: self.value & rhs.value(),
        }
    }
}

impl<F: Flag> core::ops::BitAnd<FlagSet<F>> for FlagSet<F> {
    type Output = FlagSet<F>;

    #[inline(always)]
    fn bitand(self, rhs: FlagSet<F>) -> Self::Output {
        Self {
            value: self.value() & rhs.value(),
        }
    }
}

impl<F: Flag> core::ops::BitAndAssign<F> for FlagSet<F> {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: F) {
        *self = *self & rhs;
    }
}

impl<F: Flag> core::ops::BitAndAssign<FlagSet<F>> for FlagSet<F> {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: FlagSet<F>) {
        *self = *self & rhs;
    }
}

// FSet | {F, FSet}
impl<F: Flag> core::ops::BitOr<F> for FlagSet<F> {
    type Output = FlagSet<F>;

    #[inline(always)]
    fn bitor(self, rhs: F) -> Self::Output {
        Self {
            value: self.value | rhs.value(),
        }
    }
}

impl<F: Flag> core::ops::BitOr<FlagSet<F>> for FlagSet<F> {
    type Output = FlagSet<F>;

    #[inline(always)]
    fn bitor(self, rhs: FlagSet<F>) -> Self::Output {
        Self {
            value: self.value() | rhs.value(),
        }
    }
}

impl<F: Flag> core::ops::BitOrAssign<F> for FlagSet<F> {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: F) {
        *self = *self | rhs;
    }
}

impl<F: Flag> core::ops::BitOrAssign<FlagSet<F>> for FlagSet<F> {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: FlagSet<F>) {
        *self = *self | rhs;
    }
}

// FSet ^ {F, FSet}
impl<F: Flag> core::ops::BitXor<F> for FlagSet<F> {
    type Output = FlagSet<F>;

    #[inline(always)]
    fn bitxor(self, rhs: F) -> Self::Output {
        Self {
            value: self.value ^ rhs.value(),
        }
    }
}

impl<F: Flag> core::ops::BitXor<FlagSet<F>> for FlagSet<F> {
    type Output = FlagSet<F>;

    #[inline(always)]
    fn bitxor(self, rhs: FlagSet<F>) -> Self::Output {
        Self {
            value: self.value() ^ rhs.value(),
        }
    }
}

impl<F: Flag> core::ops::BitXorAssign<F> for FlagSet<F> {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: F) {
        *self = *self ^ rhs;
    }
}

impl<F: Flag> core::ops::BitXorAssign<FlagSet<F>> for FlagSet<F> {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: FlagSet<F>) {
        *self = *self ^ rhs;
    }
}

// FSet - {F, FSet}
impl<F: Flag> core::ops::Sub<F> for FlagSet<F> {
    type Output = FlagSet<F>;
    #[inline(always)]
    fn sub(self, rhs: F) -> Self::Output {
        Self {
            value: self.value() & !rhs.value(),
        }
    }
}

impl<F: Flag> core::ops::Sub<FlagSet<F>> for FlagSet<F> {
    type Output = FlagSet<F>;

    #[inline(always)]
    fn sub(self, rhs: FlagSet<F>) -> Self::Output {
        Self {
            value: self.value() & !rhs.value(),
        }
    }
}

impl<F: Flag> core::ops::SubAssign<FlagSet<F>> for FlagSet<F> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: FlagSet<F>) {
        *self = *self - rhs;
    }
}

impl<F: Flag> core::ops::SubAssign<F> for FlagSet<F> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: F) {
        *self = *self - rhs;
    }
}

// Debug print
impl<F: Flag> core::fmt::Debug for FlagSet<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use core::write;

        write!(f, "{:#x?}", self.value())?;
        for (i, flag) in F::values().iter().enumerate() {
            if self.contains(*flag) {
                if i != 0 {
                    write!(f, " |")?;
                }
                write!(f, " {}", flag.name())?;
            }
        }
        Ok(())
    }
}

impl<F: Flag> Clone for FlagSet<F> {
    fn clone(&self) -> Self {
        FlagSet { value: self.value }
    }
}

impl<F: Flag> Copy for FlagSet<F> {}

// FSet == {F, FSet}
impl<F: Flag> core::cmp::PartialEq<F> for FlagSet<F> {
    #[inline(always)]
    fn eq(&self, other: &F) -> bool {
        self.value == other.value()
    }
}

impl<F: Flag> PartialEq for FlagSet<F> {
    fn eq(&self, other: &Self) -> bool {
        self.value() == other.value()
    }
}

impl<F: Flag> Eq for FlagSet<F> {}
