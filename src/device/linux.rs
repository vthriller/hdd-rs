use std::fs::{self, File};
use std::io;

use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader};
use std::collections::HashSet;

/// See [parent module docs](../index.html)
#[derive(Debug)]
pub struct Device {
	pub file: File,
}

#[derive(Debug)]
pub enum Type { SCSI }

impl Device {
	pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
		Ok(Device {
			file: File::open(path)?,
		})
	}

	pub fn get_type(&self) -> Result<Type, io::Error> { Ok(Type::SCSI) }

	pub fn list_devices() -> Result<Vec<PathBuf>, io::Error> {
		/*
		Various software enumerates block devices in a variety of ways:
		- smartd: probes for /dev/hd[a-t], /dev/sd[a-z], /dev/sd[a-c][a-z], /dev/nvme[0-99]
		- lsscsi: looks for *:* in /sys/bus/scsi/devices/, skipping {host,target}*
		- sg3_utils/sg_scan: iterates over /sys/class/scsi_generic if exists, otherwise probing for /dev/sg{0..8191} or /dev/sg{a..z,aa..zz,...}
		- util-linux/lsblk: iterates over /sys/block, skipping devices with major number 1 (RAM disks) by default (see --include/--exclude), as well as devices with no known size or the size of 0 (see /sys/class/block/<X>/size)
		- udisks: querying udev for devices in a "block" subsystem
		- gnome-disk-utility: just asks udisks
		- udev: just reads a bunch of files from /sys, appending irrelevant (in our case) data from hwdb and attributes set via various rules

		This code was once written using libudev, but it was dropped for a number of reason:
		- it's an extra dependency
		- it is much harder to make static builds for x86_64-unknown-linux-musl
		- it might not work on exotic systems that run mdev or rely solely on devtmpfs
		- data provided by libudev can be easily read from /sys
		- the data that libudev does not provide (e.g. `device/generic` symlink target for SCSI block devices), well, needs to be read from /sys anyways, so in a long run it's not, like, super-convenient to use this library
		*/

		let mut devices = vec![];
		let mut skip_generics = HashSet::new();

		// XXX do not return Err() if /sys/class/block does not exist but /sys/class/scsi_generic does, or vice versa

		// N.B. log entries are indented relative to each other

		info!("inspecting /sys/class/block");
		for d in fs::read_dir("/sys/class/block")? {
			let d = if let Ok(d) = d { d } else { continue };

			// XXX this assumes that dir name equals to whatever `DEVNAME` is set to in the uevent file
			// (and that `DEVNAME` is even present there)
			let name = d.file_name();
			let path = if let Ok(path) = d.path().canonicalize() { path } else {
				debug!("  {:?}: unable to read canonical device path, skipping", name);
				continue
			};
			debug!("  {:?} → {:?}", name, path);

			// skip devices like /dev/{loop,ram,zram,md,fd}*
			if path.starts_with("/sys/devices/virtual/") {
				debug!("    virtual device, skipping");
				continue;
			}
			if path.starts_with("/sys/devices/platform/floppy") {
				debug!("    floppy device, skipping");
				continue;
			}

			// $ grep -q '^DEVTYPE=disk$' /sys/class/block/sda/uevent
			if let Ok(uevent) = File::open(path.join("uevent")) {
				let mut is_disk = false;

				let mut buf = BufReader::new(uevent);
				for line in buf.lines() {
					match line {
						Ok(ref s) if s.as_str() == "DEVTYPE=disk" => {
							debug!("    {}", s);
							is_disk = true;
							break;
						}
						Ok(ref s) if s.starts_with("DEVTYPE=") => {
							debug!("    {}", s);
							is_disk = false; // see first match arm
							break;
						}
						Ok(_) => (), // keep reading
						Err(e) => {
							debug!("    problem reading uevent file: {}", e);
							break;
						},
					}
				}

				if ! is_disk {
					debug!("    undisclosed block device type, or device is not a disk, skipping");
					continue
				}
			} else {
				debug!("    unable to determine device type, skipping");
				continue;
			}

			devices.push(name);

			// e.g. `readlink /sys/class/block/sda/device/generic` → `scsi_generic/sg0`
			if let Ok(generic_path) = path.join("device/generic").read_link() {
				if let Some(generic_name) = generic_path.file_name() {
					debug!("    found corresponding scsi_generic device {:?}", generic_name);
					skip_generics.insert(generic_name.to_os_string());
				}
			}
		}

		/*
		Some drivers (e.g. aacraid) also provide generic SCSI devices for disks behind hardware RAIDs;
		these devices can be used to query SMART or SCSI logs from disks that are not represented with corresponding block devices
		*/

		info!("inspecting /sys/class/scsi_generic");
		for d in fs::read_dir("/sys/class/scsi_generic")? {
			let d = if let Ok(d) = d { d } else { continue };

			let name = d.file_name();
			debug!("  {:?}", name);

			if ! skip_generics.contains(&name) {
				devices.push(name);
			} else {
				debug!("    already covered by corresponding block device, skipping");
			}
		}

		Ok(devices
			.into_iter()
			.map(|dev| PathBuf::from(
				format!("/dev/{}", dev.into_string().unwrap())
			))
			.collect()
		)
	}
}
