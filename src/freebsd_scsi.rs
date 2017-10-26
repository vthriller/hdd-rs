extern crate libc;
use self::libc::c_void;
use std::ffi::{CStr, CString};
use std::mem;

use std::error;
use std::fmt;

extern crate cam;

use std::io::{Error, ErrorKind};

struct CAMDevice(*mut cam::cam_device);

impl CAMDevice {
	fn open(path: &str) -> Result<CAMDevice, CAMError> {
		// keep CString's buffer allocated by binding to the variable
		let path = CString::new(path).unwrap();
		let dev = unsafe { cam::cam_open_device(path.as_ptr(), libc::O_RDWR) };
		if dev.is_null() {
			Err(CAMError::current())
		} else {
			Ok(CAMDevice(dev))
		}
	}
}
impl Drop for CAMDevice {
	fn drop(&mut self) {
		unsafe {
			cam::cam_close_device(self.0);
		}
	}
}

#[derive(Debug)]
pub struct CAMError(String);
impl CAMError {
	fn current() -> CAMError { CAMError(
		unsafe {
			CStr::from_ptr(
				// strdup() to avoid implicit deallocation of external static variable
				libc::strdup(cam::cam_errbuf.as_ptr())
			).to_string_lossy().into_owned()
		}
	) }
}
impl fmt::Display for CAMError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "CAM error: {}", self.0)
	}
}
impl error::Error for CAMError {
	fn description(&self) -> &str { &self.0 }
	fn cause(&self) -> Option<&error::Error> { None }
}
// FIXME proper error types
impl From<CAMError> for Error {
	fn from(err: CAMError) -> Self {
		Error::new(ErrorKind::Other, err)
	}
}

struct CCB(*mut cam::ccb);
impl CCB {
	fn new(dev: &CAMDevice) -> CCB {
		let mut ccb: *mut cam::ccb = unsafe { cam::cam_getccb(dev.0) };

		if ccb.is_null() {
			// if we cannot allocate CCB, can we allocate something to Err()?
			panic!("cannot allocate CCB");
		}

		// it is common practice to bzero(3) non-header (ccb_hdr) part of newly allocated union
		unsafe {
			let sizeof_item = mem::size_of_val(&(*ccb).bindgen_union_field[0]);
			let start = mem::size_of::<cam::ccb_hdr>() / sizeof_item;
			let end = mem::size_of::<cam::ccb>() / sizeof_item;
			for i in start..end {
				(*ccb).bindgen_union_field[i] = 0;
			}
		}

		CCB(ccb)
	}
}
impl Drop for CCB {
	fn drop(&mut self) {
		unsafe {
			cam::cam_freeccb(self.0);
		}
	}
}

/// Executes `cmd` and puts response in the `buf`. Returns SCSI sense.
pub fn do_cmd(file: &str, cmd: &[u8], buf: &mut [u8])-> Result<[u8; 64], Error> {
	let mut sense: [u8; 64] = [0; 64];

	let dev = CAMDevice::open(file)?;
	let timeout = 10; // in seconds; TODO configurable

	let ccb: CCB = CCB::new(&dev);

	unsafe {
		let csio = (*ccb.0).csio.as_mut();

		// cannot use cam_fill_csio() here: it is defined right in cam/cam_ccb.h
		// besides, it is a pretty simple function of dubious benefit: sure it's less things to type, but with huge number of arguments it's less clear what's actually filled in a struct
		csio.ccb_h.func_code = cam::xpt_opcode::XPT_SCSI_IO;
		csio.ccb_h.flags = cam::ccb_flags::CAM_DIR_IN as u32;
		csio.ccb_h.xflags = 0;
		csio.ccb_h.retry_count = 1;
		csio.ccb_h.timeout = timeout*1000;
		csio.data_ptr = buf.as_mut_ptr();
		csio.dxfer_len = buf.len() as u32;
		csio.sense_len = 64;
		csio.tag_action = cam::MSG_SIMPLE_Q_TAG as u8;

		libc::memcpy(
			&mut csio.cdb_io.cdb_bytes as *mut _ as *mut c_void,
			cmd.as_ptr() as *const c_void,
			cmd.len(),
		);
		csio.cdb_len = cmd.len() as u8; // TODO check
	}

	if unsafe { cam::cam_send_ccb(dev.0, ccb.0) } < 0 {
		Err(CAMError::current())?
	}

	let status = unsafe { (*ccb.0).ccb_h.as_ref() }.status & cam::cam_status_CAM_STATUS_MASK as u32;
	if !(status == cam::cam_status::CAM_REQ_CMP as u32 || status == cam::cam_status::CAM_SCSI_STATUS_ERROR as u32) {
		Err(CAMError::current())?;
	}

	// TODO actual data len, data.len() - ccb.csio.resid
	// TODO ccb.csio.scsi_status
	if (status & cam::cam_status::CAM_AUTOSNS_VALID as u32) != 0 {
		// TODO actual sense len, ccb.csio.sense_len - ccb.csio.sense_resid
		unsafe { libc::memcpy(
			sense.as_mut_ptr() as *mut c_void,
			&(*ccb.0).csio.as_mut().sense_data as *const _ as *const c_void,
			64,
		) };
	}

	Ok(sense)
}
