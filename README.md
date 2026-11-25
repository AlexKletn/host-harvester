# host-harvester

[Русская версия](README.ru.md)

[Code quality](CODE_QUALITY.md) · [Commit convention](COMMIT_CONVENTIONS.md) · [Contributing](CONTRIBUTING.md) · [GPL-2.0-only](LICENSE)

## Purpose

On first run, host-harvester follows the locations and paths declared in a profile. It mounts a specified partition if needed, extracts the listed files and values, and saves the results in persistent storage. Subsequent consumers access the stored results through a host-harvester interface. The stored data remains available across reboots, independently of the original mount.

Consumers request stored items by key. Files, including binary data, are available as paths or as open read-only file descriptors. The caller owns the returned descriptor. Text values are returned as strings. For example, a board profile can extract factory Wi-Fi and Bluetooth addresses from the tablet's persist partition for other components to use.

## Consumer socket and early boot

Consumers connect to `/run/host-harvester/socket` and look up items by key. Files can be returned as paths or read-only descriptors passed over the Unix socket. Text values are returned as strings. The socket is recreated at boot. Collected data is kept on persistent storage.

For early boot, `nobscur-init` needs to make the store available and start the reader before consumers run. On first boot, it must collect and save the touch firmware before loading `nt36532`. The module currently uses Linux `request_firmware`, so an initramfs helper must retrieve the firmware from the socket and make it available under the configured firmware name in `/lib/firmware` before module load. Userspace consumers can take the descriptor directly. After switching root, the regular reader serves the same socket path.

## Run it

The profile lists mounted directories or block devices, and each entry has a unique key, a relative path, and type `file` or `text`. Omit `filesystem` for an existing directory. Set it for a block device so early boot can mount it directly through Linux without a mount helper. The [example TB371FC profile](profiles/example.toml) uses paths from the running system. In initramfs, point the firmware source at the mounted root, for example `/sysroot/usr/lib/firmware`, and make sure the block device path resolves there. The inline example assumes the external firmware was provisioned under the source directory.

```toml
[[sources]]
path = "/sysroot/opt/board-inputs"

[[sources.entries]]
key = "touch.firmware"
path = "novatek/novatek_ts_tm_fw_6.bin"
kind = "file"
```

Before loading `nt36532`, run the following with the real root mounted and the store writable. Supervise `serve` as a separate process and wait for its `ready` line before running the firmware command.

```sh
host-harvester collect /etc/host-harvester.toml /sysroot/var/lib/host-harvester /run/host-harvester/work
host-harvester serve /sysroot/var/lib/host-harvester /run/host-harvester/socket
host-harvester prepare-firmware /run/host-harvester/socket touch.firmware /lib/firmware/novatek/novatek_ts_tm_fw_6.bin
```

An import becomes visible only after every entry is saved. Failed imports can be retried on the next run. After a successful import, `collect` skips the source on later boots. Stop the early reader before switching root and start `serve /var/lib/host-harvester /run/host-harvester/socket` in the regular system. A newly created runtime directory and store are private to their owner. Consumers using the path API also need filesystem access to the store. Rust clients can use `SocketReader` and the `StorageReader` trait for paths, text, and file descriptors.

## Workspace crates

- [`hharvester-mount`](crates/hharvester-mount/)
- [`hharvester-profile`](crates/hharvester-profile/)
- [`hharvester-storage`](crates/hharvester-storage/)
- [`hharvester-storage-files`](crates/hharvester-storage-files/)
- [`hharvester-values`](crates/hharvester-values/)
- [`host-harvester`](crates/host-harvester/)

On Linux, `hharvester-mount::MountedDisk` mounts a specified block device under an existing working directory. Use `read_file` with a relative path and call `unmount` to handle cleanup errors. Dropping the handle also attempts to unmount it. Mounting requires the corresponding permissions. `mount_with_filesystem` calls Linux directly. `mount` uses the system `mount` and `umount` commands for automatic filesystem detection.
