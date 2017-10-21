use std::fs::File;
use std::os::unix::io::AsRawFd;

extern crate libc;
use self::libc::ioctl;
#[cfg(not(any(target_env = "musl")))]
use self::libc::c_ulong;
#[cfg(any(target_env = "musl"))]
use self::libc::c_int;

use std::io::Error;

use ata;

#[cfg(not(any(target_env = "musl")))]
const HDIO_DRIVE_TASK: c_ulong = 0x031e; // linux/hdreg.h:343
#[cfg(not(any(target_env = "musl")))]
const HDIO_DRIVE_CMD: c_ulong = 0x031f; // linux/hdreg.h:344

#[cfg(any(target_env = "musl"))]
const HDIO_DRIVE_TASK: c_int = 0x031e;
#[cfg(any(target_env = "musl"))]
const HDIO_DRIVE_CMD: c_int = 0x031f;

// FIXME should feature be SMARTFeature instead of u8?
pub fn ata_exec(file: &str, cmd: ata::Command, sector: u8, feature: u8, nsector: u8) -> Result<[u8; 512], Error> {
	let file = File::open(file).unwrap(); // XXX unwrap

	let mut data: [u8; 512+4] = [0; 516]; // XXX mut

	data[0] = cmd as u8;
	data[1] = sector;
	data[2] = feature;
	data[3] = nsector;

	unsafe {
		if ioctl(file.as_raw_fd(), HDIO_DRIVE_CMD, &data) == -1 {
			return Err(Error::last_os_error());
		}
		// TODO ioctl() return values other than -1?
	}

	/*
	Now, according to linux/Documentation/ioctl/hdio.txt, data contains:
		[
			status, error, nsector, _undefined,
			(nsector * 512 bytes of data returned by the command),
		]
	In practice though, first four bytes are unaltered input parameters. (XXX is it always the case?)
	*/

	// XXX mut? XXX copying
	let mut output: [u8; 512] = [0; 512];

	for i in 0..512 {
		output[i] = data[4 + i];
	}

	Ok(output)
}

// FIXME should feature be SMARTFeature instead of u8?
pub fn ata_task(file: &str, cmd: ata::Command, feature: u8, nsector: u8, sector: u8, lcyl: u8, hcyl: u8, select: u8) -> Result<ata::RegistersRead, Error> {
	let file = File::open(file).unwrap(); // XXX unwrap

	let mut data: [u8; 7] = [0; 7];

	data[0] = cmd as u8;
	data[1] = feature;
	data[2] = nsector;
	data[3] = sector;
	data[4] = lcyl;
	data[5] = hcyl;
	// > DEV bit (0x10) of SELECT register is ignored and the appropriate value for the drive is used.  All other bits are used unaltered.
	data[6] = select;

	unsafe {
		if ioctl(file.as_raw_fd(), HDIO_DRIVE_TASK, &data) == -1 {
			return Err(Error::last_os_error());
		}
	}

	Ok(ata::RegistersRead {
		status: data[0],
		error: data[1],
		sector_count: data[2],
		sector: data[3],
		cyl_low: data[4],
		cyl_high: data[5],
		device: data[6],
	})
}
