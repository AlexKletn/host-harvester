use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use hharvester_storage::StorageReader;

static NEXT_OUTPUT: AtomicU64 = AtomicU64::new(0);

pub fn prepare_firmware(
    reader: &impl StorageReader,
    key: &str,
    destination: &Path,
) -> io::Result<()> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "firmware needs an output directory",
            )
        })?;
    let mut input = File::from(reader.file_descriptor(key)?);
    fs::create_dir_all(parent)?;
    let (temporary, mut output) = loop {
        let serial = NEXT_OUTPUT.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".firmware-{}-{serial}", std::process::id()));
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o644)
            .open(&path)
        {
            Ok(file) => break (path, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };

    let result = io::copy(&mut input, &mut output)
        .and_then(|_| output.sync_all())
        .and_then(|_| fs::rename(&temporary, destination));
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
