use {eyre::Result, wordbase_api::Record};

#[derive(Debug, Clone, Default)]
pub struct Codec;

impl super::Codec for Codec {
    type Encoder = Encoder;
    type Decoder = Decoder;

    fn encoder(&self) -> Self::Encoder {
        Encoder {
            scratch: Vec::new(),
        }
    }

    fn decoder(&self) -> Self::Decoder {
        Decoder(())
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Decoder(());

impl super::Decoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record> {
        Ok(rmp_serde::from_slice(bytes)?)
    }
}
