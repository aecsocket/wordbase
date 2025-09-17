use {
    eyre::{Context, Result},
    rkyv::rancor,
    wordbase_api::{ArchivedRecord, Record},
};

pub struct Codec;

impl super::Codec for Codec {
    type Encoder = Encoder;
    type Decoder = Decoder;

    fn encoder() -> Self::Encoder {
        Encoder {}
    }

    fn decoder() -> Self::Decoder {
        Decoder(())
    }
}

pub struct Encoder {}

impl super::Encoder for Encoder {
    fn encode(&mut self, record: &Record) -> Result<impl AsRef<[u8]>> {
        let bytes = rkyv::to_bytes::<rancor::Error>(record)?;
        Ok(bytes)
    }
}

pub struct Decoder(());

impl super::Decoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record> {
        let archived = rkyv::access::<ArchivedRecord, rancor::Error>(bytes)
            .wrap_err("failed to access record")?;
        rkyv::deserialize::<Record, rancor::Error>(archived)
            .wrap_err("failed to deserialize record")
    }
}
