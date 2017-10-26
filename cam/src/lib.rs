// FIXME pub
pub mod bindings;

use std::ffi::{CStr, CString};
use std::mem;
use std::error;
use std::io::{Error, ErrorKind};
use std::fmt;

extern crate libc;

// FIXME pub cam_device
pub struct CAMDevice(pub *mut bindings::cam_device);

impl CAMDevice {
	pub fn open(path: &str) -> Result<CAMDevice, CAMError> {
		// keep CString's buffer allocated by binding to the variable
		let path = CString::new(path).unwrap();
		let dev = unsafe { bindings::cam_open_device(path.as_ptr(), libc::O_RDWR) };
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
			bindings::cam_close_device(self.0);
		}
	}
}

#[derive(Debug)]
pub struct CAMError(String);
impl CAMError {
	pub fn current() -> CAMError { CAMError(
		unsafe {
			CStr::from_ptr(
				// strdup() to avoid implicit deallocation of external static variable
				libc::strdup(bindings::cam_errbuf.as_ptr())
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

// FIXME pub ccb
pub struct CCB(pub *mut bindings::ccb);

impl CCB {
	pub fn new(dev: &CAMDevice) -> CCB {
		let mut ccb: *mut bindings::ccb = unsafe { bindings::cam_getccb(dev.0) };

		if ccb.is_null() {
			// if we cannot allocate CCB, can we allocate something to Err()?
			panic!("cannot allocate CCB");
		}

		// it is common practice to bzero(3) non-header (ccb_hdr) part of newly allocated union
		unsafe {
			let sizeof_item = mem::size_of_val(&(*ccb).bindgen_union_field[0]);
			let start = mem::size_of::<bindings::ccb_hdr>() / sizeof_item;
			let end = mem::size_of::<bindings::ccb>() / sizeof_item;
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
			bindings::cam_freeccb(self.0);
		}
	}
}
