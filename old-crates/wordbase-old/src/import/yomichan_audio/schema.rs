//! Manually reverse engineered schema.
#![expect(dead_code, reason = "we include all fields for completeness")]

// very important: trailing `/`!!!
pub const FORVO_PATH: &str = "user_files/forvo_files/";
pub const JPOD_INDEX: &str = "user_files/jpod_files/index.json";
pub const JPOD_MEDIA: &str = "user_files/jpod_files/media/";
pub const NHK16_INDEX: &str = "user_files/nhk16_files/entries.json";
pub const NHK16_AUDIO: &str = "user_files/nhk16_files/audio/";
pub const SHINMEIKAI8_INDEX: &str = "user_files/shinmeikai8_files/index.json";
pub const SHINMEIKAI8_MEDIA: &str = "user_files/shinmeikai8_files/media/";

pub const MARKER_PATHS: &[&str] = &[
    FORVO_PATH,
    JPOD_INDEX,
    JPOD_MEDIA,
    NHK16_INDEX,
    NHK16_AUDIO,
    SHINMEIKAI8_INDEX,
    SHINMEIKAI8_MEDIA,
];

pub mod generic {
    use {foldhash::HashMap, serde::Deserialize};

    #[derive(Debug, Deserialize)]
    pub struct Index {
        pub headwords: HashMap<String, Vec<String>>,
        pub files: HashMap<String, FileInfo>,
    }

    #[derive(Debug, Deserialize)]
    pub struct FileInfo {
        pub kana_reading: Option<String>,
        pub pitch_pattern: Option<String>,
        pub pitch_number: Option<String>,
    }
}

pub mod nhk16 {
    use {bytes::Bytes, serde::Deserialize};

    #[derive(Debug, Deserialize)]
    pub struct Index(pub Vec<Entry>);

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Entry {
        pub id: String,
        pub kana: String,
        pub kanji: Vec<String>,
        // Some entries contain values like this:
        //
        //     "kanjiNotUsed":["\ud842","\udf9f","咤","激","励"]
        //
        // "\ud842" is not a valid UTF-8 string - it needs a subsequent code point.
        // Which *is* given ("\udf9f"), but just not in the same string.
        // Therefore, serde fails when parsing this as a string.
        // So instead of a string, we store it as just bytes.
        pub kanji_not_used: Vec<Bytes>,
        pub kanji_raw: Vec<String>,
        pub furigana: Vec<Vec<Furigana>>,
        pub usage: Option<String>,
        pub category: Option<String>,
        pub accents: Vec<Accent>,
        pub subentries: Vec<Subentry>,
        pub examples: Vec<Example>,
        pub conjugations: Vec<Vec<OneAccent>>,
        pub references: Vec<Reference>,
        #[serde(rename = "type")]
        pub entry_type: String,
        pub class: Option<String>,
        pub notes: Vec<Note>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Furigana {
        pub character: String,
        pub reading: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Accent {
        pub not_standard_but_permissible: bool,
        #[serde(default)]
        pub accent: Vec<OneAccent>,
        pub sound_file: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OneAccent {
        // this is -1 in some cases
        pub pitch_accent: i64,
        pub silenced_mora: Vec<u64>,
        pub pronunciation: String,
        pub notes: Vec<String>,
        pub not_standard_but_permissible: bool,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Note {
        pub key: Option<String>,
        pub text: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Subentry {
        pub head: Option<String>,
        pub accents: Vec<Accent>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Example {
        pub head: Option<String>,
        pub accents: Vec<Vec<Accent>>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Reference {
        pub id: String,
        pub head: String,
    }
}
