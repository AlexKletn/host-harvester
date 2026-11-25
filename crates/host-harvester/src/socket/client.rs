use std::{
    ffi::OsString,
    io,
    os::{
        fd::{AsRawFd, OwnedFd},
        unix::ffi::OsStringExt,
    },
    path::{Path, PathBuf},
};

use hharvester_storage::StorageReader;
use hharvester_values::validate_key;
use nix::sys::socket::{UnixAddr, connect};

use super::{
    operation::Operation,
    packet::{Packet, error_kind, open_socket, set_timeouts},
};

pub struct SocketReader {
    path: PathBuf,
}

impl SocketReader {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    fn request(&self, operation: Operation, key: &str) -> io::Result<Packet> {
        validate_key(key)?;
        let socket = open_socket()?;
        set_timeouts(&socket)?;
        connect(socket.as_raw_fd(), &UnixAddr::new(&self.path)?)?;
        Packet::send(&socket, operation as u8, key.as_bytes(), None)?;
        let mut reply = Packet::receive(&socket, operation == Operation::FileDescriptor)?;
        let tag = reply.payload.remove(0);
        if tag != 0 {
            return Err(io::Error::new(
                error_kind(tag),
                String::from_utf8_lossy(&reply.payload).into_owned(),
            ));
        }

        let expects_descriptor = operation == Operation::FileDescriptor;
        if reply.descriptor.is_some() != expects_descriptor
            || (expects_descriptor && !reply.payload.is_empty())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected socket response",
            ));
        }
        Ok(reply)
    }
}

impl StorageReader for SocketReader {
    fn file_path(&self, key: &str) -> io::Result<PathBuf> {
        let reply = self.request(Operation::FilePath, key)?;
        Ok(OsString::from_vec(reply.payload).into())
    }

    fn file_descriptor(&self, key: &str) -> io::Result<OwnedFd> {
        self.request(Operation::FileDescriptor, key)?
            .descriptor
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing descriptor"))
    }

    fn read_string(&self, key: &str) -> io::Result<String> {
        String::from_utf8(self.request(Operation::Text, key)?.payload)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}
