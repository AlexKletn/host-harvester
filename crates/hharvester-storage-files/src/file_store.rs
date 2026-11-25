use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io,
    os::unix::{
        fs::{DirBuilderExt, OpenOptionsExt},
        io::OwnedFd,
    },
    path::{Path, PathBuf},
};

use hharvester_storage::StorageReader;
use hharvester_values::validate_key;

use crate::Import;

pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    pub fn open(root: &Path) -> io::Result<Self> {
        DirBuilder::new().recursive(true).mode(0o700).create(root)?;
        Ok(Self {
            root: fs::canonicalize(root)?,
        })
    }

    pub fn begin_import(&self) -> io::Result<Option<Import>> {
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(self.root.join("import.lock"))?;
        lock.lock()?;

        let published = self.root.join("data");
        match fs::symlink_metadata(&published) {
            Ok(metadata) if metadata.is_dir() => return Ok(None),
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid data directory",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        Import::new(&self.root, published, lock).map(Some)
    }

    pub fn is_ready(&self) -> bool {
        self.root.join("data").is_dir()
    }

    fn item_path(&self, directory: &str, key: &str) -> io::Result<PathBuf> {
        validate_key(key)?;
        let path = self.root.join("data").join(directory).join(key);
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stored item is not a file",
            ));
        }
        Ok(path)
    }
}

impl StorageReader for FileStore {
    fn file_path(&self, key: &str) -> io::Result<PathBuf> {
        self.item_path("files", key)
    }

    fn file_descriptor(&self, key: &str) -> io::Result<OwnedFd> {
        File::open(self.file_path(key)?).map(Into::into)
    }

    fn read_string(&self, key: &str) -> io::Result<String> {
        fs::read_to_string(self.item_path("text", key)?)
    }
}
