#![doc = include_str!("../README.md")]

mod dictionary;
mod norm_string;
mod profile;
mod protocol;
mod record;
mod term;
pub mod v1;

pub use {dictionary::*, norm_string::*, profile::*, protocol::*, record::*, term::*, uuid};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[cfg(feature = "uniffi")]
macro_rules! uuid_wrapper {
    ($ty:ident) => {
        const _: () = {
            #[derive(uniffi::Record)]
            pub struct UuidFfi {
                hi: u64,
                lo: u64,
            }

            uniffi::custom_type!($ty, UuidFfi, {
                lower: |id| {
                    let (hi, lo) = id.0.as_u64_pair();
                    UuidFfi { hi, lo }
                },
                try_lift: |ffi| Ok($ty(Uuid::from_u64_pair(ffi.hi, ffi.lo))),
            });
        };
    };
}
#[cfg(feature = "uniffi")]
pub(crate) use uuid_wrapper;
