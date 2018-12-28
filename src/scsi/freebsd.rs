use libc::{c_void, memcpy};

use cam::*;

use Direction;
use scsi::SCSIDevice;

use std::io;

impl SCSIDevice {
	/// Executes `cmd` and returns tuple of `(sense, data)`.
	pub(crate) fn do_platform_cmd(&self, cmd: &[u8], dir: Direction, sense_len: usize, data_len: usize)-> Result<(Vec<u8>, Vec<u8>), io::Error> {
		// might've used Vec::with_capacity(), but this requires rebuilding with Vec::from_raw_parts() later on to hint actual size of data in buffer vecs,
		// and we're not expecting this function to be someone's bottleneck
		let mut sense = vec![0; sense_len];
		let mut data = if let Direction::From(len) = dir {
			Some(vec![0; len])
		} else {
			None
		};

		let timeout = 10; // in seconds; TODO configurable

		let dev = &self.device.dev;

		let ccb: CCB = CCB::new(dev);

		unsafe {
			let csio = ccb.csio();

			// cannot use cam_fill_csio() here: it is defined right in cam/cam_ccb.h
			// besides, it is a pretty simple function of dubious benefit: sure it's less things to type, but with huge number of arguments it's less clear what's actually filled in a struct
			csio.ccb_h.func_code = xpt_opcode::XPT_SCSI_IO;
			csio.ccb_h.flags = {
				use self::Direction::*;
				use self::ccb_flags::*;
				match dir {
					// TODO &[u8] arg → data → csio.data_ptr for Direction::To
					From(_) => CAM_DIR_IN,
					To(_) => unimplemented!(), //CAM_DIR_OUT,
					None => CAM_DIR_NONE,
				}
			} as u32;
			csio.ccb_h.xflags = 0;
			csio.ccb_h.retry_count = 1;
			csio.ccb_h.timeout = timeout*1000;

			csio.data_ptr = match data {
				Some(ref mut data) => data.as_mut_ptr(),
				None => ptr::null_mut(),
			};
			csio.dxfer_len = match data {
				Some(ref data) => data.capacity() as u32,
				None => 0,
			};
			csio.sense_len = sense.capacity() as u8;
			csio.tag_action = MSG_SIMPLE_Q_TAG as u8;

			#[allow(trivial_casts)] // XXX
			memcpy(
				&mut csio.cdb_io.cdb_bytes as *mut _ as *mut c_void,
				cmd.as_ptr() as *const c_void,
				cmd.len(),
			);
			csio.cdb_len = cmd.len() as u8; // TODO check
		}

		dev.send_ccb(&ccb)?;

		let status = ccb.get_status();
		if !(status == cam_status::CAM_REQ_CMP as u32 || status == cam_status::CAM_SCSI_STATUS_ERROR as u32) {
			Err(error::from_status(dev, &ccb))?
		}

		// TODO ccb.csio.scsi_status

		let sense_len =
			if (ccb.get_status_flags() & cam_status::CAM_AUTOSNS_VALID as u32) != 0 {
				unsafe {
					let csio = ccb.csio();

					#[allow(trivial_casts)] // XXX
					memcpy(
						sense.as_mut_ptr() as *mut c_void,
						&csio.sense_data as *const _ as *const c_void,
						sense.capacity(),
					);

					// XXX sense_resid is always 0 to me on 11.0-RELEASE-p1 for some reason, need more testing
					// probably because I currently run this in `qemu -device lsi`
					csio.sense_len - csio.sense_resid
				}
			} else {
				0 // no valid sense, nothing to copy, sense has length 0
			};

		// TODO? return overrun flag
		// XXX > u_int32_t resid; /* Transfer residual length: 2's comp */
		// 2's comp uint?! WTF *!!*
		// XXX resid, like sense_resid, is also always 0
		let data_len = unsafe {
			ccb.csio().dxfer_len - ccb.csio().resid
		};

		Ok((
			sense[ .. sense_len as usize].to_vec(),
			// FIXME unwrap() vs Direction::{None,To}
			data.unwrap()[ .. data_len as usize].to_vec(),
		))
	}
}
