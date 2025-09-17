use {crate::codec::CodecKind, eyre::Result, wordbase_api::Record};

pub struct Codec;

impl super::Codec for Codec {
    type Encoder = Encoder;
    type Decoder = Decoder;

    fn kind() -> CodecKind {
        CodecKind::Rmp
    }

    fn encoder() -> Self::Encoder {
        Encoder {
            scratch: Vec::new(),
        }
    }

    fn decoder() -> Self::Decoder {
        Decoder(())
    }
}

pub struct Encoder {
    scratch: Vec<u8>,
}

impl super::Encoder for Encoder {
    fn encode(&mut self, record: &Record) -> Result<impl AsRef<[u8]>> {
        self.scratch.clear();
        rmp_serde::encode::write(&mut self.scratch, record)?;
        Ok(self.scratch.as_slice())
    }
}

pub struct Decoder(());

impl super::Decoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record> {
        Ok(rmp_serde::from_slice(bytes)?)
    }
}
