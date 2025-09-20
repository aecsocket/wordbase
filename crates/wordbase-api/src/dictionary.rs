use uuid::Uuid;

/// Imported collection of [`Record`]s in the engine.
///
/// This represents a dictionary which has already been imported into the
/// engine, whereas [`DictionaryMeta`] may not.
///
/// [`Record`]: crate::Record
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Object), oai(example))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Dictionary {
    /// Unique identifier for this dictionary in the database.
    pub id: DictionaryId,
    /// Meta information about this dictionary.
    pub meta: DictionaryMeta,
    /// What position [`Record`]s from this dictionary will be returned during
    /// lookups, relative to other dictionaries.
    ///
    /// A higher position means records from this dictionary will be returned
    /// later, and should be displayed to the user with a lower priority.
    ///
    /// [`Record`]: crate::Record
    pub position: i64,
}

#[cfg(feature = "poem")]
impl poem_openapi::types::Example for Dictionary {
    fn example() -> Self {
        let mut meta = DictionaryMeta::new("Jitendex");
        meta.version = Some("2025.02.11.0".into());
        meta.url = Some("https://jitendex.org".into());
        Self {
            id: DictionaryId(uuid::uuid!("6c0be404-fb5f-4f25-a9cc-6bf78667bb2b")),
            meta,
            position: 3,
        }
    }
}

/// Metadata for a [`Dictionary`].
///
/// This is `#[non_exhaustive]`: to create a new value, you must use
/// [`DictionaryMeta::new`] to create an initial value, then set fields
/// explicitly.
///
/// # Examples
///
/// ```
/// # use wordbase_api::{DictionaryMeta, DictionaryKind};
/// let mut meta = DictionaryMeta::new(DictionaryKind::Yomitan, "My Dictionary");
/// meta.version = Some("1.0.0".into());
/// meta.url = Some("https://example.com".into());
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Object))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[non_exhaustive]
pub struct DictionaryMeta {
    /// Human-readable display name.
    ///
    /// This value is **not guaranteed to be unique** across all dictionaries,
    /// however you may treat this as a stable identifier for a dictionary in
    /// its unimported form (i.e. the archive itself), and use this to detect if
    /// you attempt to import an already-imported dictionary.
    pub name: String,
    /// Arbitrary version string.
    ///
    /// This does not guarantee to conform to any format, e.g. semantic
    /// versioning.
    pub version: Option<String>,
    /// Describes the content of this dictionary.
    pub description: Option<String>,
    /// Homepage URL where users can learn more about this dictionary.
    pub url: Option<String>,
    /// Attribution information for the content of this dictionary.
    pub attribution: Option<String>,
}

impl DictionaryMeta {
    /// Creates a new value with only the required fields.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            description: None,
            url: None,
            attribution: None,
        }
    }
}

/// Opaque and unique identifier for a [`Dictionary`] in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "poem", derive(poem_openapi::NewType))]
pub struct DictionaryId(pub Uuid);

impl DictionaryId {
    /// Creates a new random ID.
    #[must_use]
    pub fn random() -> Self {
        Self(Uuid::now_v7())
    }
}

#[cfg(feature = "uniffi")]
crate::uuid_wrapper!(DictionaryId);
