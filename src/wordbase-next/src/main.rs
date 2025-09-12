//! TODO
use {
    crate::{storage::Storage, yomitan::Archive},
    eyre::{Context, Result},
    std::{io::Cursor, path::Path, time::Instant},
};

mod storage;
mod yomitan;

fn main() -> Result<()> {
    let open_archive = || {
        Ok(
            Box::new(Cursor::new(include_bytes!("../../../target/jitendex.zip")))
                as Box<dyn Archive>,
        )
    };
    let storage = storage::redb::Storage;

    let start = Instant::now();
    {
        let dir = Path::new("target/wordbase-next-db");
        std::fs::create_dir_all(dir).wrap_err("failed to create data dir")?;
        let storage = storage
            .begin_dictionary_write(dir.join("jitendex"))
            .wrap_err("failed to create dictionary storage")?;

        yomitan::finish_import(open_archive, storage).wrap_err("failed to import dictionary")?;
    }

    println!("elapsed = {:?}", start.elapsed());
    Ok(())
}
