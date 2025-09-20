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
    Rank(i64),
    /// Lower value represents a [`Term`] which appears less frequently.
    Occurrence(i64),
}
