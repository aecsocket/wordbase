use {
    crate::{DictionaryId, NormString},
    derive_more::Display,
    std::str::FromStr,
    uuid::Uuid,
};

/// Collection of user-defined settings which can be freely switched between.
///
/// The engine does not have a concept of a current profile. Instead, it is the
/// app's responsibility to manage a current profile, and pass that profile ID
/// into operations which require it (e.g. lookups).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(
    feature = "utoipa",
    derive(utoipa::ToSchema),
    schema(examples(example_profile))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Profile {
    /// Unique identifier for this profile in the database.
    pub id: ProfileId,
    /// Name of the profile.
    ///
    /// User-defined profiles will always have a name. If the name is missing,
    /// then this is the default profile made on startup, and should be labelled
    /// to the user as "Default Profile" or similar.
    pub name: Option<NormString>,
    /// Which [`Dictionary`] is used for sorting records by their frequencies.
    ///
    /// The user-set dictionary [position] always takes priority over any
    /// frequency sorting.
    ///
    /// [`Dictionary`]: crate::Dictionary
    /// [position]: crate::Dictionary::position
    pub sorting_dictionary: Option<DictionaryId>,
    /// Set of [`Dictionary`] entries which are enabled under this profile.
    ///
    /// If a dictionary is enabled, it will be used to provide results for
    /// lookups when using this profile.
    ///
    /// [`Dictionary`]: crate::Dictionary
    pub enabled_dictionaries: Vec<DictionaryId>,
}

#[cfg(feature = "utoipa")]
fn example_profile() -> Profile {
    Profile {
        id: ProfileId(uuid::uuid!("f619b6a1-28cb-4e32-8294-85b7f51f76c5")),
        name: Some(NormString::new("Japanese").expect("valid `NormString`")),
        sorting_dictionary: Some(DictionaryId(uuid::uuid!(
            "6c0be404-fb5f-4f25-a9cc-6bf78667bb2b"
        ))),
        enabled_dictionaries: vec![
            DictionaryId(uuid::uuid!("6c0be404-fb5f-4f25-a9cc-6bf78667bb2b")),
            DictionaryId(uuid::uuid!("cb5b772c-6cd7-47dd-aca8-651de6f376ae")),
        ],
    }
}

impl Profile {
    /// Creates a new profile with the default state.
    #[must_use]
    pub fn new(id: ProfileId) -> Self {
        Self {
            id,
            name: None,
            sorting_dictionary: None,
            enabled_dictionaries: Vec::new(),
        }
    }
}

/// Opaque and unique identifier for a [`Profile`] in the engine.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display("{}", _0.hyphenated())]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ProfileId(pub Uuid);

impl ProfileId {
    /// Creates a new random ID.
    #[must_use]
    pub fn random() -> Self {
        Self(Uuid::now_v7())
    }
}

impl FromStr for ProfileId {
    type Err = <Uuid as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(Self)
    }
}

#[cfg(feature = "uniffi")]
crate::uuid_wrapper!(ProfileId);
