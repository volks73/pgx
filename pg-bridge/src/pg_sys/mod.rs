//
// we allow improper_ctypes just to eliminate these warnings:
//      = note: `#[warn(improper_ctypes)]` on by default
//      = note: 128-bit integers don't currently have a known stable ABI
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(improper_ctypes)]

//
// our actual bindings modules -- these are generated by build.rs
//

mod common;

#[cfg(feature = "pg10")]
pub mod pg10_specific;

#[cfg(feature = "pg11")]
pub mod pg11_specific;

#[cfg(feature = "pg12")]
pub mod pg12_specific;

//
// publicly expose the contents of our version modules
// these are hidden behind feature gates because we can
// only expose one of these behind "pg_sys" at a time
//

#[cfg(feature = "pg10")]
pub use v10::*;

#[cfg(feature = "pg11")]
pub use v11::*;

#[cfg(feature = "pg12")]
pub use v12::*;

// version modules
// These exist to allow us to add additional items to the various version namespaces
// that couldn't be auto-generated by bindgen

/// item declarations we want to add to all versions
mod all_versions {
    use memoffset::*;

    /// this comes from `postgres_ext.h`
    pub const InvalidOid: super::Oid = 0;
    pub const InvalidOffsetNumber: super::OffsetNumber = 0;
    pub const FirstOffsetNumber: super::OffsetNumber = 1;
    pub const MaxOffsetNumber: super::OffsetNumber =
        (super::BLCKSZ as usize / std::mem::size_of::<super::ItemIdData>()) as super::OffsetNumber;
    pub const VARHDRSZ: usize = std::mem::size_of::<super::int32>();

    #[inline]
    pub fn VARHDRSZ_EXTERNAL() -> usize {
        offset_of!(super::varattrib_1b_e, va_data)
    }

    #[inline]
    pub fn VARHDRSZ_SHORT() -> usize {
        offset_of!(super::varattrib_1b, va_data)
    }
}

#[cfg(feature = "pg10")]
pub mod v10 {
    pub use super::all_versions::*;
    pub use super::common::*;

    pub type FunctionCallInfo = *mut super::pg10_specific::FunctionCallInfoData;
    pub type TupleDesc = *mut super::pg10_specific::tupleDesc;
    pub use super::pg10_specific::AllocSetContextCreate as AllocSetContextCreateExtended;
}

#[cfg(feature = "pg11")]
pub mod v11 {
    pub use super::all_versions::*;
    pub use super::common::*;

    pub type FunctionCallInfo = *mut super::pg11_specific::FunctionCallInfoData;
    pub type TupleDesc = *mut super::pg11_specific::tupleDesc;
    pub use super::pg11_specific::AllocSetContextCreateExtended;
}

#[cfg(feature = "pg12")]
pub mod v12 {
    pub use super::all_versions::*;
    pub use super::common::*;

    pub type FunctionCallInfo = *mut super::pg12_specific::FunctionCallInfoBaseData;
    pub type TupleDesc = *mut super::pg12_specific::TupleDescData;
    pub use super::pg12_specific::AllocSetContextCreateExtended;
}
