use std::{
    fs::{self, DirBuilder, File},
    io::{self, Read},
    os::unix::fs::{DirBuilderExt, FileTypeExt},
    path::{Component, Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use nix::{
    errno::Errno,
    fcntl::{OFlag, OpenHow, ResolveFlag, openat2},
};

static NEXT_MOUNT: AtomicU64 = AtomicU64::new(0);

pub struct MountedDisk {
    mount_point: PathBuf,
    mounted: bool,
    system_mount: bool,
}

impl MountedDisk {
    pub fn mount(work_dir: impl AsRef<Path>, device: impl AsRef<Path>) -> io::Result<Self> {
        Self::mount_device(work_dir.as_ref(), device.as_ref(), None)
    }

    pub fn mount_with_filesystem(
        work_dir: &Path,
        device: &Path,
        filesystem: &str,
    ) -> io::Result<Self> {
        Self::mount_device(work_dir, device, Some(filesystem))
    }

    fn mount_device(work_dir: &Path, device: &Path, filesystem: Option<&str>) -> io::Result<Self> {
        let device = fs::canonicalize(device)?;
        if !fs::metadata(&device)?.file_type().is_block_device() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected a block device",
            ));
        }

        let work_dir = fs::canonicalize(work_dir)?;
        let mount_point = create_mount_point(&work_dir)?;
        let mut disk = Self {
            mount_point,
            mounted: false,
            system_mount: filesystem.is_none(),
        };
        if let Some(filesystem) = filesystem {
            use nix::mount::{MsFlags, mount};
            let flags =
                MsFlags::MS_RDONLY | MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC;
            let options = matches!(filesystem, "ext3" | "ext4").then_some("noload");
            mount(
                Some(device.as_path()),
                &disk.mount_point,
                Some(filesystem),
                flags,
                options,
            )?;
        } else {
            let mut mount = Command::new("mount");
            mount
                .args(["--options", "ro,nodev,nosuid,noexec", "--"])
                .arg(&device)
                .arg(&disk.mount_point);
            run(&mut mount)?;
        }
        disk.mounted = true;
        Ok(disk)
    }

    pub fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    pub fn read_file(&self, relative_path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        if !self.mounted {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "disk is unmounted",
            ));
        }

        read_file_beneath(&self.mount_point, relative_path.as_ref())
    }

    pub fn unmount(&mut self) -> io::Result<()> {
        if self.mounted {
            if self.system_mount {
                run(Command::new("umount").arg("--").arg(&self.mount_point))?;
            } else {
                nix::mount::umount(&self.mount_point)?;
            }
            self.mounted = false;
        }

        match fs::remove_dir(&self.mount_point) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

impl Drop for MountedDisk {
    fn drop(&mut self) {
        let _ = self.unmount();
    }
}

fn create_mount_point(work_dir: &Path) -> io::Result<PathBuf> {
    loop {
        let serial = NEXT_MOUNT.fetch_add(1, Ordering::Relaxed);
        let path = work_dir.join(format!("disk-{}-{serial}", std::process::id()));
        match DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

pub fn read_file_beneath(root: &Path, relative_path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    open_file_beneath(root, relative_path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub fn open_file_beneath(root: &Path, relative_path: &Path) -> io::Result<File> {
    if relative_path.as_os_str().is_empty()
        || relative_path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a relative path inside the disk",
        ));
    }

    let root = File::open(root)?;
    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK | OFlag::O_NOCTTY)
        .resolve(ResolveFlag::RESOLVE_BENEATH | ResolveFlag::RESOLVE_NO_MAGICLINKS);
    let fd = openat2(&root, relative_path, how).map_err(|error| {
        if error == Errno::EXDEV {
            io::Error::new(io::ErrorKind::PermissionDenied, "path escapes the disk")
        } else {
            error.into()
        }
    })?;
    let file = File::from(fd);
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a regular file",
        ));
    }

    Ok(file)
}

fn run(command: &mut Command) -> io::Result<()> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(io::Error::other(format!(
        "{} failed ({}): {}",
        command.get_program().to_string_lossy(),
        output.status,
        String::from_utf8_lossy(&output.stderr).trim(),
    )))
}

#[cfg(test)]
#[path = "mounted_disk.tests.rs"]
mod tests;
