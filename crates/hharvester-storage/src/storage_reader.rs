use std::{io, path::PathBuf};

#[cfg(unix)]
use std::os::fd::OwnedFd;

pub trait StorageReader {
    fn file_path(&self, key: &str) -> io::Result<PathBuf>;

    #[cfg(unix)]
    fn file_descriptor(&self, key: &str) -> io::Result<OwnedFd>;

    fn read_string(&self, key: &str) -> io::Result<String>;
}
