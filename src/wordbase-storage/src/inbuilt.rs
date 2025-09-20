use {
    crate::backend::{self, Backend},
    eyre::{Result, eyre},
    serde::{Deserialize, Serialize},
    std::path::Path,
    wordbase_api::Record,
};

macro_rules! backends {
    { $(
        ( $mod:ident, $name:ident, $feature:literal )
    ),* $(,)? } => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[non_exhaustive]
        pub enum BackendKind {
            $($name,)*
        }

        #[derive(Debug)]
        #[non_exhaustive]
        pub enum InbuiltLookups {
            $(
                #[cfg(feature = $feature)]
                $name(Box<backend::$mod::Lookups>),
            )*
        }

        impl backend::Lookups for InbuiltLookups {
            fn lookup_lemma<D: crate::codec::Decoder>(
                &self,
                make_decoder: impl Fn() -> D,
                lemma: &str,
            ) -> Result<Vec<crate::RecordRow>> {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => backend::Lookups::lookup_lemma(&**this, make_decoder, lemma),
                    )*
                }
            }
        }

        pub fn open(data_dir: &Path, kind: BackendKind) -> Result<InbuiltLookups> {
            match kind {
                $(
                    #[cfg(feature = $feature)]
                    BackendKind::$name => {
                        backend::$name::open(data_dir)
                            .map(Box::new)
                            .map(InbuiltLookups::$name)
                    }
                )*
                #[allow(unreachable_patterns, reason = "depends on enabled features")]
                _ => Err(eyre!("crate was not compiled with support for backend {kind:?}")),
            }
        }
    };
}

macro_rules! codecs {
    { $(
        ( $mod:ident, $name:ident, $feature:literal )
    ),* $(,)? } => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[non_exhaustive]
        pub enum CodecKind {
            $($name,)*
        }

        #[derive(Debug)]
        #[non_exhaustive]
        pub enum InbuiltCodec {
            $(
                #[cfg(feature = $feature)]
                $name(crate::codec::$mod::Codec),
            )*
        }

        impl crate::codec::Codec for InbuiltCodec {
            type Encoder = InbuiltEncoder;
            type Decoder = InbuiltDecoder;

            fn encoder(&self) -> Self::Encoder {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => InbuiltEncoder::$name(this.encoder()),
                    )*
                }
            }

            fn decoder(&self) -> Self::Decoder {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => InbuiltDecoder::$name(this.decoder()),
                    )*
                }
            }
        }

        #[derive(Debug)]
        #[non_exhaustive]
        pub enum InbuiltEncoder {
            $(
                #[cfg(feature = $feature)]
                $name(crate::codec::$mod::Encoder),
            )*
        }

        #[non_exhaustive]
        pub enum InbuiltEncoderOutput<'enc> {
            $(
                #[cfg(feature = $feature)]
                $name(<crate::codec::$mod::Encoder as crate::codec::Encoder>::Output<'enc>),
            )*
        }

        impl AsRef<[u8]> for InbuiltEncoderOutput<'_> {
            fn as_ref(&self) -> &[u8] {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => this.as_ref(),
                    )*
                }
            }
        }

        impl crate::codec::Encoder for InbuiltEncoder {
            type Output<'enc> = InbuiltEncoderOutput<'enc>;

            fn encode(&mut self, record: &wordbase_api::Record) -> Result<Self::Output<'_>> {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => this.encode(record).map(InbuiltEncoderOutput::$name),
                    )*
                }
            }
        }

        #[derive(Debug)]
        #[non_exhaustive]
        pub enum InbuiltDecoder {
            $(
                #[cfg(feature = $feature)]
                $name(crate::codec::$mod::Decoder),
            )*
        }

        impl crate::codec::Decoder for InbuiltDecoder {
            fn decode(&mut self, bytes: &[u8]) -> Result<Record> {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => this.decode(bytes),
                    )*
                }
            }
        }

        pub fn codec(kind: CodecKind) -> Result<InbuiltCodec> {
            match kind {
                $(
                    #[cfg(feature = $feature)]
                    CodecKind::$name => Ok(InbuiltCodec::$name(crate::codec::$mod::Codec)),
                )*
                #[allow(unreachable_patterns, reason = "depends on enabled features")]
                _ => Err(eyre!("crate was not compiled with support for codec {kind:?}")),
            }
        }
    };
}

backends! {
    (heed, Heed, "backend-heed"),
    (libsql, Libsql, "backend-libsql"),
    (redb, Redb, "backend-redb"),
    (rocksdb, Rocksdb, "backend-rocksdb"),
    // (turso, Turso, "backend-turso"),
}

codecs! {
    (rmp, Rmp, "codec-rmp"),
    (rkyv, Rkyv, "codec-rkyv"),
}
