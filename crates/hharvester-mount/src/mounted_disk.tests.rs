use std::os::unix::fs::symlink;

use super::*;

#[test]
fn resolves_regular_files_and_keeps_paths_inside_the_mount_point() -> io::Result<()> {
    let work_dir = create_mount_point(&fs::canonicalize(std::env::temp_dir())?)?;
    let result = check_file_resolution(&work_dir);
    fs::remove_dir_all(work_dir)?;
    result
}

fn check_file_resolution(work_dir: &Path) -> io::Result<()> {
    let mount_point = create_mount_point(work_dir)?;
    let file = mount_point.join("inside");
    let outside = work_dir.join("outside");
    fs::write(&file, b"inside")?;
    fs::write(&outside, b"outside")?;
    symlink(&outside, mount_point.join("escape"))?;

    assert_eq!(
        read_file_beneath(&mount_point, Path::new("inside"))?,
        b"inside"
    );
    assert_eq!(
        open_file_beneath(&mount_point, Path::new("escape"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::PermissionDenied,
    );
    for invalid in [outside.as_path(), Path::new("../outside"), Path::new(".")] {
        assert_eq!(
            open_file_beneath(&mount_point, invalid).unwrap_err().kind(),
            io::ErrorKind::InvalidInput,
        );
    }
    assert_eq!(
        MountedDisk::mount(work_dir, &outside).err().unwrap().kind(),
        io::ErrorKind::InvalidInput,
    );
    Ok(())
}
