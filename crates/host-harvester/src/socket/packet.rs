use std::{
    io::{self, IoSlice, IoSliceMut},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use nix::sys::socket::{
    AddressFamily, ControlMessage, ControlMessageOwned, MsgFlags, SockFlag, SockType, recvmsg,
    sendmsg, setsockopt, socket, sockopt,
};
use nix::sys::time::TimeVal;

const MAX_PACKET: usize = hharvester_values::MAX_TEXT_BYTES + 1;

pub(crate) struct Packet {
    pub(crate) payload: Vec<u8>,
    pub(crate) descriptor: Option<OwnedFd>,
}

impl Packet {
    pub(crate) fn receive(connection: &OwnedFd, accept_descriptor: bool) -> io::Result<Self> {
        let mut payload = vec![0; MAX_PACKET];
        let mut control = nix::cmsg_space!([i32; 253]);
        let mut iov = [IoSliceMut::new(&mut payload)];
        let message = recvmsg::<()>(
            connection.as_raw_fd(),
            &mut iov,
            accept_descriptor.then_some(&mut control),
            MsgFlags::MSG_CMSG_CLOEXEC,
        )?;

        let mut descriptors = Vec::new();
        if let Ok(messages) = message.cmsgs() {
            for control in messages {
                if let ControlMessageOwned::ScmRights(fds) = control {
                    descriptors.extend(fds.into_iter().map(|fd| {
                        // SAFETY: recvmsg transferred ownership of each SCM_RIGHTS descriptor.
                        unsafe { OwnedFd::from_raw_fd(fd) }
                    }));
                }
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "truncated ancillary data",
            ));
        }
        if message.bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "empty socket packet",
            ));
        }
        if message
            .flags
            .intersects(MsgFlags::MSG_TRUNC | MsgFlags::MSG_CTRUNC)
            || descriptors.len() > 1
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid socket packet",
            ));
        }
        let bytes = message.bytes;
        payload.truncate(bytes);
        Ok(Self {
            payload,
            descriptor: descriptors.pop(),
        })
    }

    pub(crate) fn send(
        connection: &OwnedFd,
        tag: u8,
        payload: &[u8],
        descriptor: Option<&OwnedFd>,
    ) -> io::Result<()> {
        if payload.len() >= MAX_PACKET {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "socket payload is too large",
            ));
        }
        let tag = [tag];
        let iov = [IoSlice::new(&tag), IoSlice::new(payload)];
        let fds = descriptor.map(|fd| [fd.as_raw_fd()]);
        let ancillary = fds.as_ref().map(|fds| [ControlMessage::ScmRights(fds)]);
        let sent = sendmsg::<()>(
            connection.as_raw_fd(),
            &iov,
            ancillary.as_ref().map_or(&[], |messages| messages),
            MsgFlags::MSG_NOSIGNAL,
            None,
        )?;
        if sent != payload.len() + 1 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "incomplete socket packet",
            ));
        }
        Ok(())
    }
}

pub(crate) fn open_socket() -> io::Result<OwnedFd> {
    Ok(socket(
        AddressFamily::Unix,
        SockType::SeqPacket,
        SockFlag::SOCK_CLOEXEC,
        None,
    )?)
}

pub(crate) fn set_timeouts(socket: &OwnedFd) -> io::Result<()> {
    let timeout = TimeVal::new(3, 0);
    setsockopt(socket, sockopt::ReceiveTimeout, &timeout)?;
    setsockopt(socket, sockopt::SendTimeout, &timeout)?;
    Ok(())
}

pub(crate) fn error_tag(kind: io::ErrorKind) -> u8 {
    match kind {
        io::ErrorKind::NotFound => 1,
        io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => 2,
        io::ErrorKind::PermissionDenied => 3,
        _ => 4,
    }
}

pub(crate) fn error_kind(tag: u8) -> io::ErrorKind {
    match tag {
        1 => io::ErrorKind::NotFound,
        2 => io::ErrorKind::InvalidInput,
        3 => io::ErrorKind::PermissionDenied,
        _ => io::ErrorKind::Other,
    }
}
