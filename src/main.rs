use std::fs::File;
use std::os::unix::io::AsRawFd;

extern crate libc;
use libc::ioctl;
use libc::c_ulong;

use std::io::Error;

const WIN_IDENTIFY: u8 = 0xec; // linux/hdreg.h:236
const HDIO_DRIVE_CMD: c_ulong = 0x031f; // linux/hdreg.h:344

fn identify(file: File) -> Result<[u16; 256], Error> {
	let mut data: [u8; 512+4] = [0; 516]; // XXX mut

	data[0] = WIN_IDENTIFY; // command
	data[1] = 1; // nsector (sector for WIN_SMART)
	data[2] = 0; // feature
	data[3] = 1; // nsector

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

	// XXX mut?
	let mut output: [u16; 256] = [0; 256];

	for i in 0..256 {
		if cfg!(target_endian = "little") {
			output[i]  = (data[4 + 2 * i + 1] as u16) << 8;
			output[i] += data[4 + 2 * i] as u16;
		} else {
			output[i]  = (data[4 + 2 * i] as u16) << 8;
			output[i] += data[4 + 2 * i + 1] as u16;
		}
	}

	Ok(output)
}

fn read_string(arr: [u16; 256], start: usize, fin: usize) -> String {
	let mut output = String::with_capacity((fin - start) * 2);

	for i in start..fin {
		output.push((arr[i] >> 8) as u8 as char);
		output.push((arr[i] & 0xff) as u8 as char);
	}

	String::from(output.trim())
}

#[derive(Debug)]
struct Id {
	model: String,
	firmware: String,
	serial: String,
}

fn main() {
	let data = identify(
		File::open("/dev/sda").unwrap()
	).unwrap();

	print!("{:?}\n", Id {
		serial: read_string(data, 10, 19),
		firmware: read_string(data, 23, 26),
		model: read_string(data, 27, 46),
	});
}
