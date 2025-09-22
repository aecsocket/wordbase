//! See [`Rmp`].

use {eyre::Result, wordbase_core::codec, wordbase_types::Record};

/// Uses [`rmp_serde`] for serialization and deserialization.
#[derive(Debug, Clone, Default)]
pub struct Rmp;

impl codec::Codec for Rmp {
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

/// [`rmp_serde`] encoder.
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

/// [`rmp_serde`] decoder.
#[derive(Debug)]
pub struct Decoder(());

impl codec::Decoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record> {
        Ok(rmp_serde::from_slice(bytes)?)
    }
}
