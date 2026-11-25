use std::{io, path::Path};

use hharvester_mount::{MountedDisk, open_file_beneath};
use hharvester_profile::Profile;
use hharvester_storage_files::{FileStore, Import};

pub fn collect(profile_path: &Path, store: &FileStore, work_dir: &Path) -> io::Result<bool> {
    let Some(mut import) = store.begin_import()? else {
        return Ok(false);
    };

    let profile = Profile::load(profile_path)?;
    for source in profile.sources {
        if let Some(filesystem) = &source.filesystem {
            let mut disk = MountedDisk::mount_with_filesystem(work_dir, &source.path, filesystem)
                .map_err(|error| at(&source.path, error))?;
            let collected = collect_entries(&mut import, disk.mount_point(), &source.entries);
            let unmounted = disk.unmount();
            collected?;
            unmounted.map_err(|error| at(&source.path, error))?;
        } else {
            collect_entries(&mut import, &source.path, &source.entries)?;
        }
    }

    import.commit()?;
    Ok(true)
}

fn collect_entries(
    import: &mut Import,
    root: &Path,
    entries: &[hharvester_profile::Entry],
) -> io::Result<()> {
    for entry in entries {
        let file = open_file_beneath(root, &entry.path)
            .map_err(|error| at(&root.join(&entry.path), error))?;
        import
            .put(&entry.key, entry.kind, file)
            .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", entry.key)))?;
    }
    Ok(())
}

fn at(path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}
