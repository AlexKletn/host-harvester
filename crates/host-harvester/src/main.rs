use std::{
    env, fs,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

use hharvester_storage::StorageReader;
use hharvester_storage_files::FileStore;
use host_harvester::{SocketReader, SocketServer, collect, prepare_firmware};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("host-harvester: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [command, arguments @ ..] = args.as_slice() else {
        return Err(usage());
    };

    match (command.to_str(), arguments) {
        (Some("collect"), [profile, store, work_dir]) => {
            fs::create_dir_all(work_dir)?;
            if collect(
                Path::new(profile),
                &FileStore::open(Path::new(store))?,
                Path::new(work_dir),
            )? {
                println!("host-harvester: data saved");
            }
        }
        (Some("serve"), [store, socket]) => {
            let store = FileStore::open(Path::new(store))?;
            if !store.is_ready() {
                return Err(io::Error::new(io::ErrorKind::NotFound, "no collected data"));
            }
            let server = SocketServer::bind(Path::new(socket))?;
            writeln!(io::stdout().lock(), "ready")?;
            io::stdout().flush()?;
            loop {
                match server.serve_one(&store) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            }
        }
        (Some("prepare-firmware"), [socket, key, destination]) => {
            prepare_firmware(
                &SocketReader::new(socket),
                key.to_str().ok_or_else(usage)?,
                Path::new(destination),
            )?;
        }
        (Some("get-text"), [socket, key]) => {
            print!(
                "{}",
                SocketReader::new(socket).read_string(key.to_str().ok_or_else(usage)?)?
            );
        }
        (Some("get-path"), [socket, key]) => {
            println!(
                "{}",
                SocketReader::new(socket)
                    .file_path(key.to_str().ok_or_else(usage)?)?
                    .display()
            );
        }
        _ => return Err(usage()),
    }
    Ok(())
}

fn usage() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "expected collect PROFILE STORE WORK_DIR, serve STORE SOCKET, prepare-firmware SOCKET KEY PATH, get-text SOCKET KEY, or get-path SOCKET KEY",
    )
}
