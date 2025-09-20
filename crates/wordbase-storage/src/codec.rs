use {eyre::Result, wordbase_api::Record};

pub trait Codec: Send + Sync + 'static {
    type Encoder: Encoder;
    type Decoder: Decoder;

    fn encoder(&self) -> Self::Encoder;

    fn decoder(&self) -> Self::Decoder;
}

pub trait Encoder: Send + Sync + 'static {
    type Output<'enc>: AsRef<[u8]>;

    fn encode(&mut self, record: &Record) -> Result<Self::Output<'_>>;
}

pub trait Decoder: Send + Sync + 'static {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record>;
}
