use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use hharvester_values::{MAX_TEXT_BYTES, ValueKind, validate_key};

static NEXT_IMPORT: AtomicU64 = AtomicU64::new(0);

pub struct Import {
    stage: Option<PathBuf>,
    published: PathBuf,
    _lock: File,
}

impl Import {
    pub(crate) fn new(root: &Path, published: PathBuf, lock: File) -> io::Result<Self> {
        let stage = loop {
            let serial = NEXT_IMPORT.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(".import-{}-{serial}", std::process::id()));
            match DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => break path,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };
        let import = Self {
            stage: Some(stage),
            published,
            _lock: lock,
        };
        DirBuilder::new()
            .mode(0o700)
            .create(import.stage().join("files"))?;
        DirBuilder::new()
            .mode(0o700)
            .create(import.stage().join("text"))?;
        Ok(import)
    }

    pub fn put(&mut self, key: &str, kind: ValueKind, mut input: impl Read) -> io::Result<()> {
        validate_key(key)?;
        let mut text = Vec::new();
        let directory = match kind {
            ValueKind::File => "files",
            ValueKind::Text => {
                (&mut input)
                    .take((MAX_TEXT_BYTES + 1) as u64)
                    .read_to_end(&mut text)?;
                if text.len() > MAX_TEXT_BYTES {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "text item is too large",
                    ));
                }
                std::str::from_utf8(&text)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                "text"
            }
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.stage().join(directory).join(key))?;
        match kind {
            ValueKind::File => {
                io::copy(&mut input, &mut file)?;
            }
            ValueKind::Text => file.write_all(&text)?,
        }
        file.sync_all()
    }

    pub fn commit(mut self) -> io::Result<()> {
        let stage = self.stage();
        File::open(stage.join("files"))?.sync_all()?;
        File::open(stage.join("text"))?.sync_all()?;
        File::open(stage)?.sync_all()?;
        fs::rename(stage, &self.published)?;
        self.stage = None;
        File::open(self.published.parent().expect("store data has a parent"))?.sync_all()
    }

    fn stage(&self) -> &Path {
        self.stage
            .as_deref()
            .expect("uncommitted import has a staging directory")
    }
}

impl Drop for Import {
    fn drop(&mut self) {
        if let Some(stage) = &self.stage {
            let _ = fs::remove_dir_all(stage);
        }
    }
}
