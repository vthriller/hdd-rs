use std::fs::File;
use std::os::unix::io::AsRawFd;

extern crate libc;
use self::libc::ioctl;
use self::libc::c_ulong;

use std::io::Error;

const HDIO_DRIVE_CMD: c_ulong = 0x031f; // linux/hdreg.h:344

// see linux/hdreg.h
pub const WIN_IDENTIFY: u8 = 0xec;
pub const WIN_SMART: u8 = 0xb0;
pub const SMART_READ_VALUES: u8 = 0xd0;

pub fn ata_exec(file: &File, cmd: u8, sector: u8, feature: u8, nsector: u8) -> Result<[u8; 512], Error> {
	let mut data: [u8; 512+4] = [0; 516]; // XXX mut

	data[0] = cmd;
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
