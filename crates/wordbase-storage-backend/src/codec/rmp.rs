use {eyre::Result, wordbase_api::Record, wordbase_storage::codec};

#[derive(Debug, Clone, Default)]
pub struct Codec;

impl codec::Codec for Codec {
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

impl codec::Encoder for Encoder {
    type Output<'enc> = &'enc [u8];

    fn encode(&mut self, record: &Record) -> Result<Self::Output<'_>> {
        self.scratch.clear();
        rmp_serde::encode::write(&mut self.scratch, record)?;
        Ok(self.scratch.as_slice())
    }
}

#[derive(Debug)]
pub struct Decoder(());

impl codec::Decoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record> {
        Ok(rmp_serde::from_slice(bytes)?)
    }
}
