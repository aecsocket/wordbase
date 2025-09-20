#![doc = include_str!("../README.md")]

mod dictionary;
mod norm_string;
mod profile;
mod protocol;
mod record;
mod term;
pub mod v1;

use {derive_more::From, uuid::Uuid};
pub use {dictionary::*, norm_string::*, profile::*, protocol::*, record::*, term::*, uuid};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

/// Invokes a macro, passing in all existing dictionary and record kind into the
/// macro.
///
/// This serves as the source of truth for what dictionary and record kinds
/// exist in the current version of this crate. If you are adding a new kind,
/// add it here (documentation lives outside of this macro).
///
/// # Usage
///
/// Your macro will receive the following tokens:
///
/// ```text
/// $(
///     $dict_kind($dict_path) {
///         $( $record_kind ),*
///     }
/// ),*
/// ```
/// where:
/// - `$dict_kind` is:
///   - the `ident` of the [`DictionaryKind`] variant
///     - e.g. `Yomitan` maps to [`DictionaryKind::Yomitan`]
///   - the 1st half of [`RecordKind`] variant idents
///     - e.g. the `Yomitan` in [`RecordKind::YomitanGlossary`]
/// - `$dict_path` is the dictionary kind's `path` in [`dict`]
///   - e.g. `yomitan`
/// - `$record_kind` is:
///   - an `ident` of the type under `$dict_path`
///     - e.g. `Glossary` maps to `dict::yomitan::Glossary`
///   - the 2nd half of [`RecordKind`] variant idents
///     - e.g. the `Glossary` in [`RecordKind::YomitanGlossary`]
///
/// Trailing commas may be present in repetitions.
///
/// To form a [`DictionaryKind`] variant, you can use
/// `wordbase::DictionaryKind::$dict_kind`. To form a [`RecordKind`] variant,
/// you can combine `$dict_kind` and `$record_kind` via [`paste::paste`] like
/// so: `[< $dict_kind $record_kind >]`
///
/// # Examples
///
/// Generate top-level items for each record kind:
///
/// ```
/// macro_rules! define_types {
///     // copy this macro pattern exactly into your own macro
///     ($($dict_kind:ident($dict_path:ident) { $($record_kind:ident),* $(,)? }),* $(,)?) => {
///         // use `paste::paste` if you need to access record kinds
///         paste::paste! {
///             pub enum DictionaryKind {
///                 // single level of repetition here
///                 // to just iterate over the dictionary kinds
///                 $( $dict_kind, )*
///             }
///
///             pub enum RecordKind {
///                 // two levels of repetition here
///                 // to iterate over all record kinds
///                 $($( [< $dict_kind $record_kind >], )*)*
///             }
///         }
///     }
/// }
///
/// wordbase_api::for_kinds!(define_types);
/// ```
///
/// Generate code which performs the same action for all record kinds:
///
/// ```
/// # use wordbase_api::Record;
/// fn deserialize_record(kind: u32, data: &[u8]) {
///     macro_rules! deserialize_record { ($($dict_kind:ident($dict_path:ident) { $($record_kind:ident),* $(,)? }),* $(,)?) => { paste::paste! {{
///         mod discrim {
///             use wordbase_api::RecordKind;
///
///             $($(
///             pub const [< $dict_kind $record_kind >]: u32 = RecordKind::[< $dict_kind $record_kind >] as u32;
///             )*)*
///         }
///
///         match u32::try_from(kind) {
///             $($(
///             Ok(discrim::[< $dict_kind $record_kind >]) => {
///                 let record = deserialize(data);
///                 Record::[< $dict_kind $record_kind >](record)
///             }
///             )*)*
///             _ => panic!("invalid record kind {kind}"),
///         }
///     }}}}
///
///     wordbase_api::for_kinds!(deserialize_record);
/// }
/// # fn deserialize<T>(_: &[u8]) -> T { unimplemented!() }
/// ```
macro_rules! for_kinds { ($macro:ident) => { $macro!(
    Yomitan(yomitan) {
        Glossary,
        Frequency,
        Pitch,
        Phonetics,
        Kanji,
    },
    YomichanAudio(yomichan_audio) {
        Forvo,
        Jpod,
        Nhk16,
        Shinmeikai8,
    },
); } }

macro_rules! define_types { ($($dict_kind:ident($dict_path:ident) { $($record_kind:ident),* $(,)? }),* $(,)?) => { paste::paste! {

/// Kind of [`Dictionary`] that can be imported into the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Enum))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[repr(u32)]
#[non_exhaustive]
#[expect(missing_docs, reason = "self-explanatory")]
pub enum DictionaryKind {
    $($dict_kind,)*
}

impl DictionaryKind {
    /// All variants of this enum.
    pub const ALL: &[Self] = &[$(Self::$dict_kind,)*];
}

/// Kind of [`RecordKind`] that a dictionary can contain, and that a client can
/// query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Enum))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[repr(u32)]
#[non_exhaustive]
#[expect(missing_docs, reason = "self-explanatory")]
pub enum RecordKind {
    $($([< $dict_kind $record_kind >],)*)*
}

impl RecordKind {
    /// All variants of this enum.
    pub const ALL: &[Self] = &[$($(Self::[< $dict_kind $record_kind >],)*)*];
}

/// Data that a [`Dictionary`] may store for a specific [`Term`].
#[derive(Debug, Clone, From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[non_exhaustive]
#[expect(missing_docs, reason = "self-explanatory")]
pub enum Record {
    $($([< $dict_kind $record_kind >](dict::$dict_path::$record_kind),)*)*
}

impl Record {
    /// Gets the kind of this record.
    #[must_use]
    pub const fn kind(&self) -> RecordKind {
        match self {
            $($(Self::[< $dict_kind $record_kind >](_) => RecordKind::[< $dict_kind $record_kind >],)*)*
        }
    }
}

$($(
impl sealed::RecordType for dict::$dict_path::$record_kind {}

impl RecordType for dict::$dict_path::$record_kind {
    const KIND: RecordKind = RecordKind::[< $dict_kind $record_kind >];
}
)*)*

}}}
for_kinds!(define_types);

mod sealed {
    pub trait RecordType {}
}

/// Provides bounds on the type of data that can be stored in a [`Record`].
pub trait RecordType:
    sealed::RecordType + Sized + Send + Sync + std::fmt::Debug + Clone + Into<Record> + 'static
{
    /// [`RecordKind`] variant of this record type.
    const KIND: RecordKind;
}

/// Texthooker sentence event received from a [TextractorSender] server, in the
/// [exSTATic] format.
///
/// [TextractorSender]: https://github.com/KamWithK/TextractorSender
/// [exSTATic]: https://github.com/KamWithK/exSTATic/
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TexthookerSentence {
    /// Path of the process from which this texthooker sentence was extracted.
    ///
    /// This is not guaranteed to be in any format, but may be used as a
    /// persistent identifier.
    pub process_path: String,
    /// Extracted sentence.
    ///
    /// This may be malformed in some way, e.g. it may have trailing whitespace.
    pub sentence: String,
}

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
