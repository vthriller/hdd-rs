use std::fs::File;
use std::io;

extern crate libudev;
use std::path::PathBuf;

/// See [parent module docs](../index.html)
#[derive(Debug)]
pub struct Device {
	pub file: File,
}

#[derive(Debug)]
pub enum Type { SCSI }

impl Device {
	pub fn open(path: &str) -> Result<Self, io::Error> {
		Ok(Device {
			file: File::open(path)?,
		})
	}

	pub fn get_type(&self) -> Result<Type, io::Error> { Ok(Type::SCSI) }

	pub fn list_devices() -> Vec<PathBuf> {
		/*
		Various software enumerates block devices in a variety of ways:
		- smartd: probes for /dev/hd[a-t], /dev/sd[a-z], /dev/sd[a-c][a-z], /dev/nvme[0-99]
		- lsscsi: looks for *:* in /sys/bus/scsi/devices/, skipping {host,target}*
		- sg3_utils/sg_scan: iterates over /sys/class/scsi_generic if exists, otherwise probing for /dev/sg{0..8191} or /dev/sg{a..z,aa..zz,...}
		- util-linux/lsblk: iterates over /sys/block, skipping devices with major number 1 (RAM disks) by default (see --include/--exclude), as well as devices with no known size or the size of 0 (see /sys/class/block/<X>/size)
		- udisks: querying udev for devices in a "block" subsystem
		- gnome-disk-utility: just asks udisks

		Why udev?
		- easy to use
		- can possibly use some metadata from various attributes associated with devices

		Cons:
		- extra dependency
		- harder to build for x86_64-unknown-linux-musl
		- might not work on exotic systems that runn mdev or rely solely on devtmpfs
		*/

		let context = libudev::Context::new().unwrap();

		let mut enumerator = libudev::Enumerator::new(&context).unwrap();
		enumerator.match_subsystem("block").unwrap();

		return enumerator.scan_devices().unwrap()
			.filter(|d| d.is_initialized())
			// skip devices like /dev/{loop,ram,zram,md,fd}*
			.filter(|d| {
				let path = d.devpath().to_str().unwrap();
				! (path.starts_with("/devices/virtual/") || path.starts_with("/devices/platform/floppy"))
			})
			// != Some("partition")? != None? easier to just pick device types we want
			.filter(|d| d.devtype().map(|os| os.to_str().unwrap()) == Some("disk"))
			.map(|d| d.devnode().map(|path| path.to_path_buf())) // second map is because .devnode() returns &Path that is owned by temporary udev::Device
				.filter(|d| d.is_some()).map(|d| d.unwrap())
			.collect();
	}
}
