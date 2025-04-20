use {
    foldhash::HashMap,
    serde::{Deserialize, Serialize, de::DeserializeOwned},
    std::fmt::Debug,
};

pub trait Request: Send + Sync + Debug + Serialize {
    type Response: DeserializeOwned;

    const ACTION: &'static str;
    const HAS_PARAMS: bool;
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestWrapper<'r, R> {
    pub version: u32,
    pub action: &'r str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<&'r R>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version;

impl Request for Version {
    type Response = u32;

    const ACTION: &str = "version";
    const HAS_PARAMS: bool = false;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckNames;

impl Request for DeckNames {
    type Response = Vec<DeckName>;

    const ACTION: &str = "deckNames";
    const HAS_PARAMS: bool = false;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeckName(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelNames;

impl Request for ModelNames {
    type Response = Vec<ModelName>;

    const ACTION: &str = "modelNames";
    const HAS_PARAMS: bool = false;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelName(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFieldNames {
    pub model_name: ModelName,
}

impl Request for ModelFieldNames {
    type Response = Vec<ModelFieldName>;

    const ACTION: &str = "modelFieldNames";
    const HAS_PARAMS: bool = true;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelFieldName(pub String);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddNote<'a> {
    pub note: Note<'a>,
}

impl Request for AddNote<'_> {
    type Response = NoteId;

    const ACTION: &'static str = "addNote";
    const HAS_PARAMS: bool = true;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note<'a> {
    pub deck_name: &'a str,
    pub model_name: &'a str,
    pub fields: HashMap<&'a str, &'a str>,
    pub options: NoteOptions<'a>,
    pub tags: Vec<&'a str>,
    pub audio: Vec<Asset<'a>>,
    pub video: Vec<Asset<'a>>,
    pub picture: Vec<Asset<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteOptions<'a> {
    pub allow_duplicate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_scope: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_scope_options: Option<DuplicateScopeOptions<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScopeOptions<'a> {
    pub deck_name: &'a str,
    pub check_children: bool,
    pub check_all_models: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset<'a> {
    pub filename: &'a str,
    pub data: Option<&'a str>,
    pub path: Option<&'a str>,
    pub url: Option<&'a str>,
    pub skip_hash: Option<&'a str>,
    pub fields: Vec<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteId(pub u64);
