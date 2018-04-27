use cam::bindings;
use cam::ccb::CCB;
use cam::error;
use std::io;

use std::ffi::{OsStr, CString};
use std::os::unix::ffi::OsStrExt;

use libc;

/// Thin wrapper for `struct cam_device *`. Note that the underlying raw pointer is *mutable*.
///
/// This struct implements `Drop`, i.e. you don't need to call `cam_close_device` yourself.
#[derive(Debug)]
pub struct CAMDevice(pub *mut bindings::cam_device);

impl CAMDevice {
	pub fn open(path: &OsStr) -> Result<Self, io::Error> {
		// keep CString's buffer allocated by binding to the variable
		let path = CString::new(path.as_bytes()).unwrap();
		let dev = unsafe { bindings::cam_open_device(path.as_ptr(), libc::O_RDWR) };
		if dev.is_null() {
			Err(error::current())
		} else {
			Ok(CAMDevice(dev))
		}
	}

	pub fn send_ccb(&self, ccb: &CCB) -> Result<(), io::Error> {
		if unsafe { bindings::cam_send_ccb(self.0, ccb.0) } < 0 {
			Err(io::Error::last_os_error())
		} else { Ok(()) }
	}
}

impl Drop for CAMDevice {
	fn drop(&mut self) {
		unsafe { bindings::cam_close_device(self.0); }
	}
}
