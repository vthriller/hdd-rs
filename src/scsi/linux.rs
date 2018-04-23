use libc::{c_int, c_uint, c_uchar, c_ushort, c_void};

use libc::ioctl;
use std::ptr;

#[cfg(not(any(target_env = "musl")))]
use libc::c_ulong;

use std::os::unix::io::AsRawFd;
use std::io;

use Direction;
use scsi::SCSIDevice;

use std::cmp::max;

// see scsi/sg.h

#[cfg(not(any(target_env = "musl")))]
const SG_IO: c_ulong = 0x2285;

#[cfg(any(target_env = "musl"))]
const SG_IO: c_int = 0x2285;

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

impl SCSIDevice {
	pub(crate) fn do_platform_cmd(&self, cmd: &[u8], dir: Direction, sense_len: usize, data_len: usize) -> Result<(Vec<u8>, Vec<u8>), io::Error> {
		// might've used Vec::with_capacity(), but this requires rebuilding with Vec::from_raw_parts() later on to hint actual size of data in buffer vecs,
		// and we're not expecting this function to be someone's bottleneck
		let mut sense = vec![0; sense_len];
		let mut data = vec![0; data_len];

		let hdr = sg_io_hdr {
			interface_id:	'S' as c_int,

			dxfer_direction: match dir {
				// see scsi/sg.h, constants SG_DXFER_{NONE,{TO,FROM,TO_FROM}_DEV}
				// TODO &[u8] arg → data → sg_io_hdr.dxferp for Direction::{To,Both}
				Direction::None => -1,
				Direction::To => unimplemented!(), //-2,
				Direction::From => -3,
				Direction::Both => unimplemented!(), //-4,
			},
			dxferp:	data.as_mut_ptr() as *mut c_void,
			dxfer_len:	data.capacity() as c_uint,
			resid:	0,

			sbp:	sense.as_mut_ptr(),
			mx_sb_len:	sense.capacity() as c_uchar,
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
			if ioctl(self.device.file.as_raw_fd(), SG_IO, &hdr) == -1 {
				return Err(io::Error::last_os_error());
			}
		}

		// > In practice [resid] only reports underruns (i.e. positive number) as data overruns should never happen
		// but I'd still not cast i32 to u32 blindly, just to be sure
		// TODO? return overrun flag
		// XXX sg_io set resid to 0 for SATA disks, and Hitachi SAS disks behind Adaptec also set this to 0 for things like LOG SENSE 0fh/00h—need more reading/testing
		let data_len = hdr.dxfer_len - max(hdr.resid, 0) as u32;

		Ok((
			sense[ .. hdr.sb_len_wr as usize].to_vec(),
			data[ .. data_len as usize].to_vec(),
		))
	}
}
