use std::{
    fs::{self, DirBuilder, File, OpenOptions, Permissions},
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{DirBuilderExt, FileTypeExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
};

use hharvester_storage::StorageReader;
use hharvester_values::validate_key;
use nix::sys::socket::{Backlog, SockFlag, UnixAddr, accept4, bind, listen};

use super::{
    operation::Operation,
    packet::{Packet, error_tag, open_socket, set_timeouts},
};

pub struct SocketServer {
    listener: OwnedFd,
    path: PathBuf,
    _lock: File,
}

impl SocketServer {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "socket needs a directory")
            })?;
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        let mut lock_path = path.as_os_str().to_owned();
        lock_path.push(".lock");
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(lock_path)?;
        lock.try_lock().map_err(io::Error::from)?;

        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_socket() => {
                let existing = open_socket()?;
                match nix::sys::socket::connect(existing.as_raw_fd(), &UnixAddr::new(path)?) {
                    Err(nix::errno::Errno::ECONNREFUSED) => fs::remove_file(path)?,
                    Err(error) => return Err(error.into()),
                    Ok(()) => {
                        return Err(io::Error::new(
                            io::ErrorKind::AddrInUse,
                            "socket has a listener",
                        ));
                    }
                }
            }
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "socket path is occupied",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        let listener = open_socket()?;
        bind(listener.as_raw_fd(), &UnixAddr::new(path)?)?;
        let server = Self {
            listener,
            path: path.to_owned(),
            _lock: lock,
        };
        fs::set_permissions(path, Permissions::from_mode(0o660))?;
        listen(&server.listener, Backlog::new(16)?)?;
        Ok(server)
    }

    pub fn serve_one(&self, store: &impl StorageReader) -> io::Result<()> {
        let fd = accept4(self.listener.as_raw_fd(), SockFlag::SOCK_CLOEXEC)?;
        // SAFETY: accept4 returned a newly owned socket descriptor.
        let connection = unsafe { OwnedFd::from_raw_fd(fd) };
        if let Err(error) = serve_client(&connection, store) {
            let message = error.to_string();
            let _ = Packet::send(
                &connection,
                error_tag(error.kind()),
                message.as_bytes(),
                None,
            );
        }
        Ok(())
    }
}

impl Drop for SocketServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn serve_client(connection: &OwnedFd, store: &impl StorageReader) -> io::Result<()> {
    set_timeouts(connection)?;
    let request = Packet::receive(connection, false)?;
    if request.descriptor.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "request contains a descriptor",
        ));
    }

    let operation = Operation::try_from(request.payload[0])?;
    let key = std::str::from_utf8(&request.payload[1..])
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    validate_key(key)?;
    match operation {
        Operation::FilePath => {
            let path = store.file_path(key)?;
            Packet::send(connection, 0, path.as_os_str().as_encoded_bytes(), None)
        }
        Operation::FileDescriptor => {
            let fd = store.file_descriptor(key)?;
            Packet::send(connection, 0, &[], Some(&fd))
        }
        Operation::Text => {
            let text = store.read_string(key)?;
            Packet::send(connection, 0, text.as_bytes(), None)
        }
    }
}
