mod yomitan;

use tokio::sync::{mpsc, oneshot};
pub use yomitan::yomitan;

use derive_more::{Display, Error};
use wordbase::DictionaryMeta;

#[derive(Debug, Clone, Display, Error)]
#[display("already exists")]
pub struct AlreadyExists;

#[derive(Debug)]
pub struct ReadToMemory {
    pub recv_read_meta: oneshot::Receiver<ReadMeta>,
}

#[derive(Debug)]
pub struct ReadMeta {
    pub meta: DictionaryMeta,
    pub banks_len: usize,
    pub recv_banks_left: mpsc::Receiver<usize>,
    pub recv_parsed: oneshot::Receiver<Parsed>,
}

#[derive(Debug)]
pub struct Parsed {
    pub records_len: usize,
    pub recv_records_left: mpsc::Receiver<usize>,
    pub recv_inserted: oneshot::Receiver<()>,
}
