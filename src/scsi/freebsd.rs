extern crate libc;
use self::libc::c_void;

use cam::*;

use Direction;
use Device;
use scsi::SCSIDevice;

use std::io::Error;

// TODO reindent
impl SCSIDevice for Device {

fn do_cmd(&self, cmd: &[u8], dir: Direction, buf: &mut [u8])-> Result<[u8; 64], Error> {
	let mut sense: [u8; 64] = [0; 64];

	let timeout = 10; // in seconds; TODO configurable

	let ccb: CCB = CCB::new(&self.dev);

	unsafe {
		let csio = ccb.csio();

		// cannot use cam_fill_csio() here: it is defined right in cam/cam_ccb.h
		// besides, it is a pretty simple function of dubious benefit: sure it's less things to type, but with huge number of arguments it's less clear what's actually filled in a struct
		csio.ccb_h.func_code = xpt_opcode::XPT_SCSI_IO;
		csio.ccb_h.flags = match dir {
			Direction::From => ccb_flags::CAM_DIR_IN,
			Direction::To => ccb_flags::CAM_DIR_OUT,
			Direction::Both => ccb_flags::CAM_DIR_BOTH,
			Direction::None => ccb_flags::CAM_DIR_NONE,
		} as u32;
		csio.ccb_h.xflags = 0;
		csio.ccb_h.retry_count = 1;
		csio.ccb_h.timeout = timeout*1000;
		csio.data_ptr = buf.as_mut_ptr();
		csio.dxfer_len = buf.len() as u32;
		csio.sense_len = 64;
		csio.tag_action = MSG_SIMPLE_Q_TAG as u8;

		libc::memcpy(
			&mut csio.cdb_io.cdb_bytes as *mut _ as *mut c_void,
			cmd.as_ptr() as *const c_void,
			cmd.len(),
		);
		csio.cdb_len = cmd.len() as u8; // TODO check
	}

	self.dev.send_ccb(&ccb)?;

	let status = ccb.get_status();
	if !(status == cam_status::CAM_REQ_CMP as u32 || status == cam_status::CAM_SCSI_STATUS_ERROR as u32) {
		Err(CAMError::current())?;
	}

	// TODO actual data len, data.len() - ccb.csio.resid
	// TODO ccb.csio.scsi_status
	if (status & cam_status::CAM_AUTOSNS_VALID as u32) != 0 {
		// TODO actual sense len, ccb.csio.sense_len - ccb.csio.sense_resid
		unsafe { libc::memcpy(
			sense.as_mut_ptr() as *mut c_void,
			&ccb.csio().sense_data as *const _ as *const c_void,
			64,
		) };
	}

	Ok(sense)
}

}
