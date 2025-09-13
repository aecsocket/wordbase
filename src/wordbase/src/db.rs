use {
    redb::{MultimapTableDefinition, TableDefinition},
    std::{any, cmp},
    wordbase_api::Record,
};

pub const RECORDS: TableDefinition<RecordId, &[u8]> = TableDefinition::new("records");
pub const HEADWORDS: MultimapTableDefinition<&str, RecordId> =
    MultimapTableDefinition::new("headwords");
pub const READINGS: MultimapTableDefinition<&str, RecordId> =
    MultimapTableDefinition::new("readings");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordId(pub u64);

impl redb::Value for RecordId {
    type SelfType<'a> = Self;
    type AsBytes<'a> = <u64 as redb::Value>::AsBytes<'a>;

    fn fixed_width() -> Option<usize> {
        <u64 as redb::Value>::fixed_width()
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        Self(<u64 as redb::Value>::from_bytes(data))
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        <u64 as redb::Value>::as_bytes(&value.0)
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new(any::type_name::<Self>())
    }
}

impl redb::Key for RecordId {
    fn compare(data1: &[u8], data2: &[u8]) -> cmp::Ordering {
        <u64 as redb::Key>::compare(data1, data2)
    }
}
