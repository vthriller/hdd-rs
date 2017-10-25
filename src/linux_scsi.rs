extern crate libc;
use self::libc::{c_int, c_uint, c_uchar, c_ushort, c_void};

use self::libc::ioctl;
use std::ptr;

#[cfg(not(any(target_env = "musl")))]
use self::libc::c_ulong;

use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::io::Error;

// see scsi/sg.h

#[cfg(not(any(target_env = "musl")))]
const SG_IO: c_ulong = 0x2285;

#[cfg(any(target_env = "musl"))]
const SG_IO: c_int = 0x2285;

const SG_DXFER_NONE: c_int = -1;
const SG_DXFER_TO_DEV: c_int = -2;
const SG_DXFER_FROM_DEV: c_int = -3;
const SG_DXFER_TO_FROM_DEV: c_int = -4;

#[repr(C)]
#[derive(Debug)]
struct sg_io_hdr {
	interface_id:	c_int,	// [i] 'S' for SCSI generic (required)
	dxfer_direction:	c_int,	// [i] data transfer direction
	cmd_len:	c_uchar,	// [i] SCSI command length ( <= 16 bytes)
	mx_sb_len:	c_uchar,	// [i] max length to write to sbp
	iovec_count:	c_ushort,	// [i] 0 implies no scatter gather
	dxfer_len:	c_uint,	// [i] byte count of data transfer
	dxferp:	*mut c_void,	// [i], [*io] points to data transfer memory or scatter gather list
	cmdp:	*const c_uchar,	// [i], [*i] points to command to perform
	sbp:	*mut c_uchar,	// [i], [*o] points to sense_buffer memory
	timeout:	c_uint,	// [i] MAX_UINT->no timeout (unit: millisec)
	flags:	c_uint,	// [i] 0 -> default, see SG_FLAG...
	pack_id:	c_int,	// [i->o] unused internally (normally)
	usr_ptr:	*mut c_void,	// [i->o] unused internally
	status:	c_uchar,	// [o] scsi status
	masked_status:	c_uchar,	// [o] shifted, masked scsi status
	msg_status:	c_uchar,	// [o] messaging level data (optional)
	sb_len_wr:	c_uchar,	// [o] byte count actually written to sbp
	host_status:	c_ushort,	// [o] errors from host adapter
	driver_status:	c_ushort,	// [o] errors from software driver
	resid:	c_int,	// [o] dxfer_len - actual_transferred
	duration:	c_uint,	// [o] time taken by cmd (unit: millisec)
	info:	c_uint,	// [o] auxiliary information
}

/// Executes `cmd` and puts response in the `buf`. Returns SCSI sense.
pub fn do_cmd(file: &str, cmd: &[u8], buf: &mut [u8])-> Result<[u8; 64], Error> {
	let file = File::open(file).unwrap(); // XXX unwrap

	let mut sense: [u8; 64] = [0; 64];

	let hdr = sg_io_hdr {
		interface_id:	'S' as c_int,

		dxfer_direction:	SG_DXFER_FROM_DEV, // TODO
		dxferp:	buf.as_mut_ptr() as *mut c_void,
		dxfer_len:	buf.len() as c_uint,
		resid:	0,

		sbp:	sense.as_mut_ptr(),
		mx_sb_len:	sense.len() as c_uchar,
		sb_len_wr:	0,

		cmdp:	cmd.as_ptr(),
		cmd_len:	cmd.len() as c_uchar,

		status:	0,
		host_status:	0,
		driver_status:	0,

		timeout:	10000,	// TODO configurable
		duration:	0,

		iovec_count:	0,
		flags:	0,
		pack_id:	0,
		usr_ptr:	ptr::null_mut(),
		masked_status:	0,
		msg_status:	0,
		info:	0,
	};

	unsafe {
		if ioctl(file.as_raw_fd(), SG_IO, &hdr) == -1 {
			return Err(Error::last_os_error());
		}
	}

	Ok(sense)
}
