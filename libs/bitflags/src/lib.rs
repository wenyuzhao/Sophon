#![no_std]
#![feature(const_mut_refs)]

use core::{
    fmt,
    ops::{BitAnd, BitOr, BitXor},
};

pub trait BitFlag: Sized + Clone + Copy + PartialEq {
    type Repr: fmt::LowerHex
        + BitAnd<Output = Self::Repr>
        + BitOr<Output = Self::Repr>
        + BitXor<Output = Self::Repr>
        + Default
        + PartialEq
        + Clone
        + Copy;
    const ZERO: Self::Repr;
    #[inline(always)]
    fn bits(self) -> Self::Repr {
        unsafe { *(&self as *const Self as *const Self::Repr) }
    }
    #[inline(always)]
    fn flags(self) -> BitFlags<Self> {
        BitFlags::from_flag(self)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct BitFlags<F: BitFlag>(F::Repr);

impl<F: BitFlag> fmt::LowerHex for BitFlags<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BitFlags({:#x})", self.0)
    }
}

impl<F: BitFlag> BitFlags<F> {
    pub fn from_flag(flag: F) -> Self {
        Self(unsafe { *(&flag as *const F as *const F::Repr) })
    }

    pub fn from_flags(flags: &[F]) -> Self {
        let mut flagset = Self(F::ZERO);
        for f in flags {
            flagset = flagset | *f;
        }
        flagset
    }

    // pub const fn from_flags(flags: &[F]) -> Self {
    //     // if flags.len() == 0 {
    //     //     // let
    //     //     // Self(unsafe { ::core::mem::transmute(0) })
    //     //     unreachable!()
    //     // } else if flags.len() == 1 {
    //     //     Self::from_flag(&flags[0])
    //     // } else {
    //     //     let mut f = Self::from_flag(&flags[0]);
    //     //     Self(f.bits() | Self::from_flags(&flags[1..]).bits())
    //     // }
    //     let mut i = 0;
    //     let mut f = F::ZERO;
    //     while i < flags.len() {
    //         f = f | Self::from_flag(&flags[i]).bits();
    //         i += 1;
    //     }
    //     Self::from_flag(&flags[0])
    // }

    pub const fn from_bits(bits: F::Repr) -> Self {
        Self(bits)
    }

    pub const fn set_bits(&mut self, bits: F::Repr) {
        self.0 = bits;
    }

    // pub const fn add_flag(&mut self, flag: F) {
    //     self.set_bits(self.bits() | Self::from_flag(flag).bits());
    // }

    pub const fn bits(&self) -> F::Repr {
        self.0
    }

    pub fn contains(&self, flag: F) -> bool {
        (flag.bits() & self.0) != F::Repr::default()
    }
}

impl<F: BitFlag> BitAnd for BitFlags<F> {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.bits() & rhs.bits())
    }
}

impl<F: BitFlag> BitOr for BitFlags<F> {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.bits() | rhs.bits())
    }
}

impl<F: BitFlag> BitXor for BitFlags<F> {
    type Output = Self;

    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self {
        Self(self.bits() ^ rhs.bits())
    }
}

impl<F: BitFlag> BitAnd<F> for BitFlags<F> {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: F) -> Self {
        Self(self.bits() & rhs.bits())
    }
}

impl<F: BitFlag> BitOr<F> for BitFlags<F> {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: F) -> Self {
        Self(self.bits() | rhs.bits())
    }
}

impl<F: BitFlag> BitXor<F> for BitFlags<F> {
    type Output = Self;

    #[inline(always)]
    fn bitxor(self, rhs: F) -> Self {
        Self(self.bits() ^ rhs.bits())
    }
}

// #[macro_export]
// macro_rules! const_bitflags {
//     ($($flag: expr),*) => {{
//         let mut flags = BitFlags::from_bits(0);
//         $({
//             flags.add_flag($flag);
//         });*
//         flags
//     }};
// }
