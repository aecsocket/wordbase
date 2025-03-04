use bytes::Bytes;
use derive_more::{Display, Error, From};
use serde::{Deserialize, Serialize};

use crate::{
    SharedConfig,
    dict::{Dictionary, DictionaryId, ExpressionEntry},
};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum FromClient {
    #[from]
    NewSentence(NewSentence),
    Lookup {
        text: String,
    },
    ListDictionaries,
    RemoveDictionary {
        dictionary_id: DictionaryId,
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
    SyncConfig {
        config: SharedConfig,
    },
    NewSentence(NewSentence),
    Lookup {
        lookup: Option<LookupInfo>,
    },
    ListDictionaries {
        dictionaries: Vec<Dictionary>,
    },
    RemoveDictionary {
        result: Result<(), DictionaryNotFound>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupInfo {
    pub lemma: String,
    pub expressions: Vec<ExpressionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stage")]
pub enum ImportStage {
    Received,
    Done { result: Result<(), ()> },
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
        round_trip(FromClient::ListDictionaries);
        round_trip(FromClient::RemoveDictionary {
            dictionary_id: default(),
        });

        round_trip(FromServer::Error { message: default() });
        round_trip(FromServer::SyncConfig { config: default() });
        round_trip(FromServer::NewSentence(NewSentence::default()));
        round_trip(FromServer::Lookup { lookup: None });
        round_trip(FromServer::Lookup {
            lookup: Some(LookupInfo {
                lemma: default(),
                expressions: default(),
            }),
        });
        round_trip(FromServer::ListDictionaries {
            dictionaries: vec![Dictionary::default()],
        });
        round_trip(FromServer::RemoveDictionary { result: Ok(()) });
        round_trip(FromServer::RemoveDictionary {
            result: Err(DictionaryNotFound),
        });
    }
}
