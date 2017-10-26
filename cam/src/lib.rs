//! Thin wrapper against FreeBSD's libcam.
//!
//! This module is not intended to be a full-featured wrapper even for a subset of all the things that libcam provides.
//! Instead, this module offers a number of helpers and shortcuts, like `impl Drop`, that should aid with writing a bit more concise and idiomatic code against libcam.
//! Users of this module are expected to do most of the things on their own, manually, using things like `unsafe {}` and `.0`.
//!
//! This module is not for a general use. Besides the fact that it is utterly incomplete and unfriendly, this binding also lacks a lot of things that libcam provides, as they are irrelevant to the purposes of the crate.
//! For the list of exported FFI interfaces, consult `cam/build.rs`.
//!
//! For more on CAM, see [cam(4)][man-cam], [FreeBSD Architecture Handbook][arch-handbook-scsi], [The Design and Implementation of the FreeBSD SCSI Subsystem][difss], or just lurk around [source files in e.g. /usr/src/sbin/camcontrol/][camcontrol-svn].
//!
//! [man-cam]: https://www.freebsd.org/cgi/man.cgi?query=cam&apropos=0&sektion=4&manpath=FreeBSD+11.1-RELEASE+and+Ports&arch=default&format=html
//! [arch-handbook-scsi]: https://www.freebsd.org/doc/en_US.ISO8859-1/books/arch-handbook/scsi.html
//! [difss]: https://people.freebsd.org/~gibbs/ARTICLE-0001.html
//! [camcontrol-svn]: https://svnweb.freebsd.org/base/stable/11/sbin/camcontrol/

pub mod bindings;

pub use bindings::{
	CAM_ATAIO_48BIT,
	CAM_ATAIO_NEEDRESULT,
	MSG_SIMPLE_Q_TAG,
	cam_status,
	ccb_flags,
	xpt_opcode,
};

use std::ffi::{CStr, CString};
use std::mem;
use std::error;
use std::io::{Error, ErrorKind};
use std::fmt;

extern crate libc;

/// Thin wrapper for `struct cam_device *`. Note that the underlying raw pointer is *mutable*.
///
/// This struct implements `Drop`, i.e. you don't need to call `cam_close_device` yourself.
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

	pub fn send_ccb(self, ccb: &CCB) -> Result<(), CAMError> {
		if unsafe { bindings::cam_send_ccb(self.0, ccb.0) } < 0 {
			Err(CAMError::current())
		} else { Ok(()) }
	}
}

impl Drop for CAMDevice {
	fn drop(&mut self) {
		unsafe {
			bindings::cam_close_device(self.0);
		}
	}
}

/// Regular error type for CAM-related actions. In case of emergency, just do
///
/// ```
/// Err(CAMError::current())?
/// ```
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

/// Thin wrapper for `union ccb *`, the CAM Control Block. Note that the underlying raw pointer is *mutable*.
///
/// This struct implements `Drop`, i.e. you don't need to call `cam_freeccb` yourself.
pub struct CCB(pub *mut bindings::ccb);

impl CCB {
	/// Calls `cam_getccb` with provided device, and zeroes all the values except for those that compose its header (`ccb_h`), which is a common pratice (see smartmontools, camcontrol).
	///
	/// # Panics
	///
	/// This function panics if `cam_getccb` returns `NULL`, assuming there's not enough memory to allocate anything, not even `Err` to return.
	pub fn new(dev: &CAMDevice) -> CCB {
		let mut ccb: *mut bindings::ccb = unsafe { bindings::cam_getccb(dev.0) };

		if ccb.is_null() {
			panic!("cannot allocate CCB");
		}

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
	pub fn get_status(&self) -> u32 {
		unsafe {
			(*self.0).ccb_h.as_ref()
		}.status & bindings::cam_status_CAM_STATUS_MASK as u32
	}
	// those are deliberately kept unsafe
	pub unsafe fn ccb_h(&self) -> &mut bindings::ccb_hdr { (*self.0).ccb_h.as_mut() }
	pub unsafe fn csio(&self) -> &mut bindings::ccb_scsiio { (*self.0).csio.as_mut() }
	pub unsafe fn ataio(&self) -> &mut bindings::ccb_ataio { (*self.0).ataio.as_mut() }
}

impl Drop for CCB {
	fn drop(&mut self) {
		unsafe {
			bindings::cam_freeccb(self.0);
		}
	}
}
