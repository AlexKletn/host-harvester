use std::{
    fs::{self, DirBuilder, File},
    io::{self, Read},
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use hharvester_storage::StorageReader;
use hharvester_storage_files::FileStore;
use host_harvester::{SocketReader, SocketServer, collect, prepare_firmware};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> io::Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("hharvester-{}-{timestamp}", std::process::id()));
        DirBuilder::new().mode(0o700).create(&root)?;
        Ok(Self(root))
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn imports_atomically_and_reads_firmware_by_descriptor() -> io::Result<()> {
    let root = TestDirectory::new()?;
    let input = root.0.join("source");
    let profile = root.0.join("profile.toml");
    let stored_data = root.0.join("persistent");
    let work = root.0.join("work");
    let socket = root.0.join("run/socket");
    let firmware = root.0.join("initramfs/lib/firmware/novatek/touch.bin");
    fs::create_dir(&input)?;
    fs::create_dir(&work)?;
    let bytes = b"\x00\x01\xff firmware";
    fs::write(input.join("touch.bin"), bytes)?;
    fs::write(
        &profile,
        format!(
            "[[sources]]\npath = {:?}\n\n[[sources.entries]]\nkey = \"touch.firmware\"\npath = \"touch.bin\"\nkind = \"file\"\n\n[[sources.entries]]\nkey = \"wifi.address\"\npath = \"wifi\"\nkind = \"text\"\n",
            input.to_str().unwrap()
        ),
    )?;

    let store = FileStore::open(&stored_data)?;
    assert_eq!(
        collect(&profile, &store, &work).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    assert!(!store.is_ready());

    fs::write(input.join("wifi"), "02:11:22:33:44:55")?;
    assert!(collect(&profile, &store, &work)?);
    fs::remove_dir_all(&input)?;
    assert!(!collect(&profile, &store, &work)?);
    drop(store);
    let store = FileStore::open(&stored_data)?;

    let server = SocketServer::bind(&socket)?;
    let client = SocketReader::new(&socket);
    thread::scope(|threads| -> io::Result<()> {
        threads.spawn(|| {
            for _ in 0..5 {
                server.serve_one(&store).unwrap();
            }
        });
        assert_eq!(client.read_string("wifi.address")?, "02:11:22:33:44:55");
        assert_eq!(
            fs::read(client.file_path("touch.firmware")?)?,
            bytes.as_slice()
        );
        let mut descriptor = File::from(client.file_descriptor("touch.firmware")?);
        let mut received = Vec::new();
        descriptor.read_to_end(&mut received)?;
        assert_eq!(received, bytes);
        assert_eq!(
            client.file_path("missing").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        prepare_firmware(&client, "touch.firmware", &firmware)?;
        Ok(())
    })?;
    assert_eq!(fs::read(firmware)?, bytes);
    drop(server);
    assert!(!socket.exists());
    assert!(SocketServer::bind(&socket).is_ok());
    Ok(())
}
