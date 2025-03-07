use derive_more::{Display, Error, From};
use serde::{Deserialize, Serialize};

use crate::{
    Dictionary, DictionaryId,
    lookup::{LookupConfig, LookupEntry},
};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum FromClient {
    #[from]
    NewSentence(NewSentence),
    Lookup {
        text: String,
    },
    RemoveDictionary {
        dictionary_id: DictionaryId,
    },
    SetDictionaryEnabled {
        dictionary_id: DictionaryId,
        enabled: bool,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewSentence {
    pub process_path: String,
    pub sentence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FromServer {
    Error {
        message: String,
    },
    SyncLookupConfig {
        lookup_config: LookupConfig,
    },
    SyncDictionaries {
        dictionaries: Vec<Dictionary>,
    },
    NewSentence(NewSentence),
    Lookup {
        entries: Vec<LookupEntry>,
    },
    RemoveDictionary {
        result: Result<(), DictionaryNotFound>,
    },
    SetDictionaryEnabled {
        result: Result<(), DictionaryNotFound>,
    },
}

#[derive(Debug, Clone, Display, Error, Serialize, Deserialize)]
#[display("dictionary not found")]
pub struct DictionaryNotFound;

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;

    use super::*;

    fn default<T: Default>() -> T {
        T::default()
    }

    fn round_trip<T: Serialize + DeserializeOwned>(original: T) {
        let json = serde_json::to_string_pretty(&original).unwrap();
        println!("{json}");
        serde_json::from_str::<T>(&json).unwrap();
    }

    #[test]
    fn round_trip_all() {
        round_trip(FromClient::from(NewSentence::default()));
        round_trip(FromClient::Lookup { text: default() });
        round_trip(FromClient::RemoveDictionary {
            dictionary_id: default(),
        });
        round_trip(FromClient::SetDictionaryEnabled {
            dictionary_id: default(),
            enabled: default(),
        });

        round_trip(FromServer::Error { message: default() });
        round_trip(FromServer::SyncLookupConfig {
            lookup_config: default(),
        });
        round_trip(FromServer::SyncDictionaries {
            dictionaries: vec![default()],
        });
        round_trip(FromServer::NewSentence(NewSentence::default()));
        round_trip(FromServer::Lookup {
            entries: vec![default()],
        });
        round_trip(FromServer::RemoveDictionary { result: Ok(()) });
        round_trip(FromServer::RemoveDictionary {
            result: Err(DictionaryNotFound),
        });
        round_trip(FromServer::SetDictionaryEnabled { result: Ok(()) });
        round_trip(FromServer::SetDictionaryEnabled {
            result: Err(DictionaryNotFound),
        });
    }
}
