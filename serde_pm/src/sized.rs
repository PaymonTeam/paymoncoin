//! `PMSized` trait for any Rust data structure a predictable size of its PM binary
//! representation can be computed.
//!
//! # Examples
//!
//! ```
//! use serde_pm::{PMSized, ByteBuf};
//!
//! struct Something {
//!     name: String,
//!     small_num: u16,
//!     raw_data: ByteBuf,
//!     pair: (i8, u64),
//! }
//!
//! // Implement manually
//!
//! impl PMSized for Something {
//!     fn size_hint(&self) -> serde_pm::Result<usize> {
//!         let mut result = 0;
//!
//!         result += self.name.size_hint()?;
//!         result += self.small_num.size_hint()?;
//!         result += self.raw_data.size_hint()?;
//!         result += self.pair.size_hint()?;
//!
//!         Ok(result)
//!     }
//! }
//!
//! # fn run() -> serde_pm::Result<()> {
//! let smth = Something {
//!     name: "John Smith".to_owned(),
//!     small_num: 2000u16,
//!     raw_data: ByteBuf::from(vec![0xf4, 0x58, 0x2e, 0x33]),
//!     pair: (-50, 0xffff_ffff_ffff_ffff),
//! };
//!
//! // "John Smith" => 1 byte length, 10 bytes data, 1 byte padding;
//! // 2000u16 => 4 bytes;
//! // ByteBuf { ... } => 1 byte length, 4 bytes data, 3 bytes padding;
//! // (-50, 0xffff_ffff_ffff_ffff) => 4 bytes + 8 bytes == 12 bytes;
//! //
//! // Total: 12 + 4 + 8 + 12 == 36 bytes
//!
//! assert_eq!(36, smth.size_hint()?);
//! #     Ok(())
//! # }
//!
//! # fn main() { run().unwrap(); }
//! ```
//!
//! Alternatively, `PMSized` can be `#[derive]`d:
//!
//! ```
//! #[macro_use]
//! extern crate serde_pm_derive;
//!
//! #[derive(PMSized)]
//! struct Something {
//!     name: String,
//!     small_num: u16,
//!     raw_data: Vec<u8>,
//!     pair: (i8, u64),
//! }
//!
//! # fn main() {}
//! ```
//!
//! The derived implementation is the same as the one shown above.

use std::collections::{HashMap, BTreeMap};
use std::hash::{BuildHasher, Hash};

//use serde_bytes::{ByteBuf, Bytes};

use error::{self, SerializationError};
use utils::check_seq_len;


/// Size of a bool PM value.
pub const BOOL_SIZE: usize = 4;
/// Size of an byte PM value.
pub const BYTE_SIZE: usize = 1;
/// Size of an int PM value.
pub const INT_SIZE: usize = 4;
/// Size of a long PM value.
pub const LONG_SIZE: usize = 8;
/// Size of a float PM value.
pub const FLOAT_SIZE: usize = 4;
/// Size of a double PM value.
pub const DOUBLE_SIZE: usize = 8;
/// Size of an int128 PM value.
pub const INT128_SIZE: usize = 16;


/// A trait for a Rust data structure a predictable size of its PM binary representation
/// can be computed.
pub trait PMSized {
    /// Compute the size of PM binary representation of this value without actually
    /// serializing it.
    ///
    /// Returns an `error::Result` because not any value can be serialized (e.g. strings and
    /// sequences that are too long).
    fn size_hint(&self) -> error::Result<usize>;
}


macro_rules! impl_pm_sized_for_primitives {
    ($($type:ty => $size:expr,)+) => {
        $(
            impl PMSized for $type {
                fn size_hint(&self) -> error::Result<usize> {
                    Ok($size)
                }
            }
        )+
    };
}

impl_pm_sized_for_primitives! {
    bool => BOOL_SIZE,

    i8  => BYTE_SIZE,
    i16 => INT_SIZE,
    i32 => INT_SIZE,
    i64 => LONG_SIZE,

    u8  => BYTE_SIZE,
    u16 => INT_SIZE,
    u32 => INT_SIZE,
    u64 => LONG_SIZE,

    f32 => FLOAT_SIZE,
    f64 => DOUBLE_SIZE,
}

#[cfg(stable_i128)]
impl_pm_sized_for_primitives! {
    i128 => INT128_SIZE,
    u128 => INT128_SIZE,
}


/// Helper function for everything naturally representable as a byte sequence.
///
/// This version **does take** into account the byte sequence length, which is prepended to the
/// serialized representation of the byte sequence.
pub fn size_hint_from_byte_seq_len(len: usize) -> error::Result<usize> {
    let (len_info, data, padding) = if len <= 253 {
        (1, len, (4 - (len + 1) % 4) % 4)
    } else if len <= 0xff_ff_ff {
        (4, len, (4 - len % 4) % 4)
    } else {
        bail!(SerializationError::SeqTooLong);
    };

    let size = len_info + data + padding;
    assert_eq!(size % 4, 0);

    Ok(size)
}

impl<'a> PMSized for &'a str {
    fn size_hint(&self) -> error::Result<usize> {
        size_hint_from_byte_seq_len(self.as_bytes().len())
    }
}

impl PMSized for String {
    fn size_hint(&self) -> error::Result<usize> {
        size_hint_from_byte_seq_len(self.as_bytes().len())
    }
}

impl<'a, T: ?Sized + PMSized> PMSized for &'a T {
    fn size_hint(&self) -> error::Result<usize> {
        (*self).size_hint()
    }
}

impl<T: ?Sized + PMSized> PMSized for Box<T> {
    fn size_hint(&self) -> error::Result<usize> {
        (**self).size_hint()
    }
}

impl<'a, T: PMSized> PMSized for &'a [T] {
    fn size_hint(&self) -> error::Result<usize> {
        // If len >= 2 ** 32, it's not serializable at all.
        check_seq_len(self.len())?;
        let mut result = 4;    // 4 for slice length

        for elem in self.iter() {
            result += elem.size_hint()?;
        }

        // Check again just to be sure
        check_seq_len(result)?;

        Ok(result)
    }
}

impl<T: PMSized> PMSized for Vec<T> {
    fn size_hint(&self) -> error::Result<usize> {
        self.as_slice().size_hint()
    }
}

impl<K, V, S> PMSized for HashMap<K, V, S>
    where K: Eq + Hash + PMSized,
          V: PMSized,
          S: BuildHasher,
{
    fn size_hint(&self) -> error::Result<usize> {
        // If len >= 2 ** 32, it's not serializable at all.
        check_seq_len(self.len())?;

        let mut result = 4;    // 4 for map length

        for (k, v) in self.iter() {
            result += k.size_hint()?;
            result += v.size_hint()?;
        }

        // Check again just to be sure
        check_seq_len(result)?;

        Ok(result)
    }
}

impl<K, V> PMSized for BTreeMap<K, V>
    where K: PMSized,
          V: PMSized,
{
    fn size_hint(&self) -> error::Result<usize> {
        // If len >= 2 ** 32, it's not serializable at all.
        check_seq_len(self.len())?;

        let mut result = 4;    // 4 for map length

        for (k, v) in self.iter() {
            result += k.size_hint()?;
            result += v.size_hint()?;
        }

        // Check again just to be sure
        check_seq_len(result)?;

        Ok(result)
    }
}

impl PMSized for () {
    fn size_hint(&self) -> error::Result<usize> {
        Ok(0)
    }
}

//impl<'a> PMSized for Bytes<'a> {
//    fn size_hint(&self) -> error::Result<usize> {
//        size_hint_from_byte_seq_len(self.len())
//    }
//}
//
//impl PMSized for ByteBuf {
//    fn size_hint(&self) -> error::Result<usize> {
//        size_hint_from_byte_seq_len(self.len())
//    }
//}

macro_rules! impl_pm_sized_for_tuple {
    ($($ident:ident : $ty:ident ,)+) => {
        impl<$($ty),+> PMSized for ($($ty,)+)
            where $($ty: PMSized,)+
        {
            fn size_hint(&self) -> error::Result<usize> {
                let mut result = 0;
                let ($(ref $ident,)+) = *self;
                $( result += $ident.size_hint()?; )+
                info!("TUPLE SIze");
                Ok(result)
            }
        }
    };
}

impl_pm_sized_for_tuple! { x1: T1, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, x6: T6, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, x6: T6, x7: T7, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, x6: T6, x7: T7, x8: T8, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, x6: T6, x7: T7, x8: T8,
                                 x9: T9, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, x6: T6, x7: T7, x8: T8,
                                 x9: T9, x10: T10, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, x6: T6, x7: T7, x8: T8,
                                 x9: T9, x10: T10, x11: T11, }
impl_pm_sized_for_tuple! { x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, x6: T6, x7: T7, x8: T8,
                                 x9: T9, x10: T10, x11: T11, x12: T12, }

macro_rules! impl_pm_sized_for_arrays {
    (__impl 0) => {
        impl<T> PMSized for [T; 0] {
            fn size_hint(&self) -> error::Result<usize> {
                Ok(0)
            }
        }
    };

    (__impl $size:expr) => {
        impl<T: PMSized> PMSized for [T; $size] {
            fn size_hint(&self) -> error::Result<usize> {
                let mut result = 0;

                for elem in self {
                    result += elem.size_hint()?;
                }

                Ok(result)
            }
        }
    };

    ($($size:expr),+) => {
        $( impl_pm_sized_for_arrays!(__impl $size); )+
    };
}

impl_pm_sized_for_arrays!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);
