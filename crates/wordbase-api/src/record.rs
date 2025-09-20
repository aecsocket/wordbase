use crate::v1;

macro_rules! record_kinds {
    ($($data:path => $kind:ident),* $(,)?) => {

/// Kind of [`Record`] that a dictionary can contain, and that a client can
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
    $($kind,)*
}

/// Data that a [`Dictionary`] may store for a specific [`Term`].
///
/// [`Dictionary`]: crate::Dictionary
/// [`Term`]: crate::Term
#[derive(Debug, Clone, derive_more::From)]
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
    $($kind($data),)*
}

impl Record {
    /// Gets the kind of this record.
    #[must_use]
    pub const fn kind(&self) -> RecordKind {
        match self {
            $(Self::$kind(_) => RecordKind::$kind,)*
        }
    }
}

$(
impl sealed::RecordType for $data {}

impl RecordType for $data {
    const KIND: RecordKind = RecordKind::$kind;
}
)*

    };
}

record_kinds! {
    v1::yomitan::Glossary => YomitanGlossaryV1,
    v1::yomitan::Frequency => YomitanFrequencyV1,
    v1::yomitan::Pitch => YomitanPitchV1,
    v1::yomitan::Phonetics => YomitanPhoneticsV1,
    v1::yomitan::Kanji => YomitanKanjiV1,
    v1::yomichan_audio::Forvo => YomichanAudioForvoV1,
    v1::yomichan_audio::Jpod => YomichanAudioJpodV1,
    v1::yomichan_audio::Nhk16 => YomichanAudioNhk16V1,
    v1::yomichan_audio::Shinmeikai8 => YomichanAudioShinmeikai8V1,
}

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

/// Opaque and unique identifier for a [`Record`] in the engine.
///
/// Multiple [`Term`]s may link to a single [`Record`].
///
/// [`Term`]: crate::Term
/// [`Record`]: crate::Record
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "poem", derive(poem_openapi::NewType))]
pub struct RecordId(pub u64);

#[cfg(feature = "uniffi")]
uniffi::custom_newtype!(RecordId, u64);

/// How often a [`Term`] appears in a single [`Dictionary`].
///
/// This value is used for sorting lookup results. However, the value given is
/// only valid in the context of a **single specific** [`Dictionary`]. That is,
/// if you take a [`FrequencyValue`] from one [`Dictionary`] and compare it to
/// another [`FrequencyValue`] from a different [`Dictionary`], the result is
/// meaningless.
///
/// There is explicitly no way to get the [`i64`] from this value while ignoring
/// the variant, as the value does not make sense without knowing if it's a rank
/// or an occurrence.
///
/// [`Term`]: crate::Term
/// [`Dictionary`]: crate::Dictionary
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum FrequencyValue {
    /// Lower value represents a [`Term`] which appears more frequently.
    ///
    /// [`Term`]: crate::Term
    Rank(i64),
    /// Lower value represents a [`Term`] which appears less frequently.
    ///
    /// [`Term`]: crate::Term
    Occurrence(i64),
}
