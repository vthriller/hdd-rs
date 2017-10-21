extern crate libc;
use self::libc::{c_int, c_uint, c_uchar, c_ushort, c_void};

use self::libc::ioctl;
use std::{mem, ptr};

#[cfg(not(any(target_env = "musl")))]
use self::libc::c_ulong;

use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::io::{Error, ErrorKind};

use ata;

// see scsi/sg.h

#[cfg(not(any(target_env = "musl")))]
const SG_IO: c_ulong = 0x2285;

#[cfg(any(target_env = "musl"))]
const SG_IO: c_int = 0x2285;

pub const SG_DXFER_NONE: c_int = -1;
pub const SG_DXFER_TO_DEV: c_int = -2;
pub const SG_DXFER_FROM_DEV: c_int = -3;
pub const SG_DXFER_TO_FROM_DEV: c_int = -4;

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

fn ata_pass_through_16(file: &str, regs: &ata::RegistersWrite) -> Result<([u8; 64], [u8; 512]), Error> {
	let file = File::open(file).unwrap(); // XXX unwrap

	// see T10/04-262r8a ATA Command Pass-Through, 3.2.3
	let extend = 0; // TODO
	let protocol = 4; // PIO Data-In; TODO
	let multiple_count = 0; // TODO
	let ata_cmd: [u8; 16] = [
		0x85, // opcode: ATA PASS-THROUGH (16)
		(multiple_count << 5) + (protocol << 1) + extend,
		// 0b00: wait up to 2^(OFF_LINE+1)-2 seconds for valid ATA status register
		// 0b1: CK_COND, return ATA register info in the sense data
		// 0b0: reserved
		// 0b1: T_DIR; transfer from ATA device
		// 0b1: BYT_BLOK; T_LENGTH is in blocks, not in bytes
		// 0b01: T_LENGTH itself
		0b00101101,
		0, regs.features,
		0, regs.sector_count,
		0, regs.sector,
		0, regs.cyl_low,
		0, regs.cyl_high,
		regs.device,
		regs.command,
		0, // control (XXX what's that?!)
	];

	let mut sense: [u8; 64] = [0; 64];
	let mut buf: [u8; 512] = [0; 512];

	let hdr = sg_io_hdr {
		interface_id:	'S' as c_int,

		dxfer_direction:	SG_DXFER_FROM_DEV, // TODO
		dxferp:	buf.as_mut_ptr() as *mut c_void,
		dxfer_len:	mem::size_of_val(&buf) as c_uint,
		resid:	0,

		sbp:	sense.as_mut_ptr(),
		mx_sb_len:	mem::size_of_val(&sense) as c_uchar,
		sb_len_wr:	0,

		cmdp:	ata_cmd.as_ptr(),
		cmd_len:	mem::size_of_val(&ata_cmd) as c_uchar,

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

	Ok((sense, buf))
}

pub fn ata_pass_through_16_exec(file: &str, regs: &ata::RegistersWrite) -> Result<[u8; 512], Error> {
	// FIXME: yep, those are pre-filled for users of HDIO_DRIVE_CMD ioctl
	let regs = if regs.command == ata::Command::SMART as u8 {
		ata::RegistersWrite { cyl_low: 0x4f, cyl_high: 0xc2, ..*regs }
	} else {
		ata::RegistersWrite { ..*regs }
	};

	let (_, buf) = ata_pass_through_16(file, &regs)?;
	Ok(buf)
}

pub fn ata_pass_through_16_task(file: &str, regs: &ata::RegistersWrite) -> Result<ata::RegistersRead, Error> {
	let (sense, _) = ata_pass_through_16(file, regs)?;

	if sense[0] & 0x7f != 0x72 {
		// we expected current sense in the descriptor format
		// TODO proper error
		return Err(Error::new(ErrorKind::Other, "unexpected sense format/whatever"));
	}

	// sense[7] is the additional sense length; in other words, it's what amount of data descriptors occupy
	let sense_length = 8 + sense[7] as usize;
	let mut current_desc: usize = 8;
	// iterate over descriptors
	while current_desc < sense_length {
		let (code, length) = (sense[current_desc], sense[current_desc + 1]);
		if !(code == 0x09 && length == 12) {
			// descriptor is not about ATA Status or is malformed (invalid length)
			current_desc += length as usize;
			continue;
		}
		// TODO? EXTEND bit, ATA PASS-THROUGH 12 vs 16
		return Ok(ata::RegistersRead {
			error: sense[current_desc + 3],

			sector_count: sense[current_desc + 5],

			sector: sense[current_desc + 7],
			cyl_low: sense[current_desc + 9],
			cyl_high: sense[current_desc + 11],
			device: sense[current_desc + 12],

			status: sense[current_desc + 13],
		})
	}

	// TODO proper error
	return Err(Error::new(ErrorKind::Other, "no (valid) sense descriptors found"));
}
