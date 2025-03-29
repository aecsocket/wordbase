use std::collections::HashMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub trait Request: Serialize {
    type Response: DeserializeOwned;

    const ACTION: &str;
    const HAS_PARAMS: bool;
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestWrapper<'r, R> {
    pub version: u32,
    pub action: &'r str,
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
pub struct AddNote {
    pub note: Note,
}

impl Request for AddNote {
    type Response = NoteId;

    const ACTION: &str = "addNote";
    const HAS_PARAMS: bool = true;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub deck_name: String,
    pub model_name: String,
    pub fields: HashMap<String, String>,
    pub options: NoteOptions,
    pub tags: Vec<String>,
    pub audio: Vec<Asset>,
    pub video: Vec<Asset>,
    pub picture: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteOptions {
    pub allow_duplicate: bool,
    pub duplicate_scope: String,
    pub duplicate_scope_options: Option<DuplicateScopeOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScopeOptions {
    pub deck_name: String,
    pub check_children: bool,
    pub check_all_models: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub url: String,
    pub filename: String,
    pub skip_hash: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteId(pub u64);
