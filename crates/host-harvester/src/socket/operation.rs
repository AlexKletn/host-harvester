use std::io;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Operation {
    FilePath = 1,
    FileDescriptor = 2,
    Text = 3,
}

impl TryFrom<u8> for Operation {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        match value {
            1 => Ok(Self::FilePath),
            2 => Ok(Self::FileDescriptor),
            3 => Ok(Self::Text),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unknown operation",
            )),
        }
    }
}
