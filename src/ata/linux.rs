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

pub fn ata_exec(file: &str, regs: &ata::RegistersWrite) -> Result<[u8; 512], Error> {
	let file = File::open(file).unwrap(); // XXX unwrap

	let mut data: [u8; 512+4] = [0; 516]; // XXX mut

	data[0] = regs.command;
	data[1] = regs.sector;
	data[2] = regs.features;
	data[3] = regs.sector_count;
	// XXX cyl_low cyl_high device are filled for us
	// for ata::Command::SMART, cyl_[lh] are 0x4f 0xc2, for the rest they're undefined
	// and device is 0xa0|DEV_bit|LBA_bit except in rare cases

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

pub fn ata_task(file: &str, regs: &ata::RegistersWrite) -> Result<ata::RegistersRead, Error> {
	let file = File::open(file).unwrap(); // XXX unwrap

	let mut data: [u8; 7] = [0; 7];

	data[0] = regs.command;
	data[1] = regs.features;
	data[2] = regs.sector_count;
	data[3] = regs.sector;
	data[4] = regs.cyl_low;
	data[5] = regs.cyl_high;
	// XXX > DEV bit (0x10) of SELECT register is ignored and the appropriate value for the drive is used.  All other bits are used unaltered.
	data[6] = regs.device;

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
