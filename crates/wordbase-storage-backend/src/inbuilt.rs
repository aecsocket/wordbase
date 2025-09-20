use {
    eyre::{Result, eyre},
    serde::{Deserialize, Serialize},
    std::path::Path,
    wordbase_api::Record,
    wordbase_storage::backend::RecordRow,
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
                $name(Box<crate::backend::$mod::Lookups>),
            )*
        }

        impl wordbase_storage::backend::Lookups for InbuiltLookups {
            fn lookup_lemma<D: wordbase_storage::codec::Decoder>(
                &self,
                make_decoder: impl Fn() -> D,
                lemma: &str,
            ) -> Result<Vec<RecordRow>> {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => wordbase_storage::backend::Lookups::lookup_lemma(&**this, make_decoder, lemma),
                    )*
                    #[allow(unreachable_patterns, reason = "depends on enabled features")]
                    _ => unreachable!(),
                }
            }
        }

        pub fn open(data_dir: &Path, kind: BackendKind) -> Result<InbuiltLookups> {
            match kind {
                $(
                    #[cfg(feature = $feature)]
                    BackendKind::$name => {
                        <crate::backend::$name as wordbase_storage::backend::Backend>::open(data_dir)
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

        impl wordbase_storage::codec::Codec for InbuiltCodec {
            type Encoder = InbuiltEncoder;
            type Decoder = InbuiltDecoder;

            fn encoder(&self) -> Self::Encoder {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => InbuiltEncoder::$name(this.encoder()),
                    )*
                    #[allow(unreachable_patterns, reason = "depends on enabled features")]
                    _ => unreachable!(),
                }
            }

            fn decoder(&self) -> Self::Decoder {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => InbuiltDecoder::$name(this.decoder()),
                    )*
                    #[allow(unreachable_patterns, reason = "depends on enabled features")]
                    _ => unreachable!(),
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
                $name(<crate::codec::$mod::Encoder as wordbase_storage::codec::Encoder>::Output<'enc>),
            )*
        }

        impl AsRef<[u8]> for InbuiltEncoderOutput<'_> {
            fn as_ref(&self) -> &[u8] {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => this.as_ref(),
                    )*
                    #[allow(unreachable_patterns, reason = "depends on enabled features")]
                    _ => unreachable!(),
                }
            }
        }

        impl wordbase_storage::codec::Encoder for InbuiltEncoder {
            type Output<'enc> = InbuiltEncoderOutput<'enc>;

            fn encode(&mut self, record: &wordbase_api::Record) -> Result<Self::Output<'_>> {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => this.encode(record).map(InbuiltEncoderOutput::$name),
                    )*
                    #[allow(unreachable_patterns, reason = "depends on enabled features")]
                    _ => unreachable!(),
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

        impl wordbase_storage::codec::Decoder for InbuiltDecoder {
            fn decode(&mut self, bytes: &[u8]) -> Result<Record> {
                match self {
                    $(
                        #[cfg(feature = $feature)]
                        Self::$name(this) => this.decode(bytes),
                    )*
                    #[allow(unreachable_patterns, reason = "depends on enabled features")]
                    _ => unreachable!(),
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
