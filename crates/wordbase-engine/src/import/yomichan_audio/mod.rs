use anyhow::Result;
use bytes::Bytes;

use super::ImportFormat;

pub struct YomichanAudio;

impl ImportFormat for YomichanAudio {
    async fn validate(archive: Bytes) -> Result<()> {}
}
