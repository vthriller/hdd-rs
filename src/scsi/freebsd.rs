extern crate libc;
use self::libc::c_void;

use cam::*;

use Direction;
use Device;
use scsi::SCSIDevice;

use std::io::Error;

impl SCSIDevice for Device {
	fn do_cmd(&self, cmd: &[u8], dir: Direction, buf: &mut [u8], sense_len: u8)-> Result<Vec<u8>, Error> {
		// might've used Vec::with_capacity(), but this requires rebuilding with Vec::from_raw_parts() later on to hint actual size of a sense,
		// and we're not expecting this function to be someone's bottleneck
		let mut sense = vec![0; sense_len as usize];

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
			csio.sense_len = sense.capacity() as u8;
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

		let sense_len =
			if (ccb.get_status_flags() & cam_status::CAM_AUTOSNS_VALID as u32) != 0 {
				unsafe {
					let csio = ccb.csio();

					libc::memcpy(
						sense.as_mut_ptr() as *mut c_void,
						&csio.sense_data as *const _ as *const c_void,
						sense.capacity(),
					);

					// XXX sense_resid is always 0 to me on 11.0-RELEASE-p1 for some reason, need more testing
					csio.sense_len - csio.sense_resid
				}
			} else {
				0 // no valid sense, nothing to copy, sense has length 0
			};

		Ok(sense[ .. sense_len as usize].to_vec())
	}
}
