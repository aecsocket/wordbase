use {
    eyre::{Context, Result},
    rkyv::{rancor, util::AlignedVec},
    wordbase_api::{ArchivedRecord, Record},
    wordbase_storage::codec,
};

#[derive(Debug, Clone, Default)]
pub struct Codec;

impl codec::Codec for Codec {
    type Encoder = Encoder;
    type Decoder = Decoder;

    fn encoder(&self) -> Self::Encoder {
        Encoder {}
    }

    fn decoder(&self) -> Self::Decoder {
        Decoder(())
    }
}

#[derive(Debug)]
pub struct Encoder {}

impl codec::Encoder for Encoder {
    type Output<'enc> = AlignedVec;

    fn encode(&mut self, record: &Record) -> Result<Self::Output<'_>> {
        let bytes = rkyv::to_bytes::<rancor::Error>(record)?;
        Ok(bytes)
    }
}

#[derive(Debug)]
pub struct Decoder(());

impl codec::Decoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record> {
        let archived = rkyv::access::<ArchivedRecord, rancor::Error>(bytes)
            .wrap_err("failed to access record")?;
        rkyv::deserialize::<Record, rancor::Error>(archived)
            .wrap_err("failed to deserialize record")
    }
}
