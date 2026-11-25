#[cfg(target_os = "linux")]
mod mounted_disk;

#[cfg(target_os = "linux")]
pub use mounted_disk::{MountedDisk, open_file_beneath, read_file_beneath};
