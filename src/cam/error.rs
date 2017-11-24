use cam::bindings;
use cam::device::CAMDevice;
use cam::ccb::CCB;

use std::ffi::CStr;
use std::error;
use std::io;
use std::fmt;

extern crate libc;

/// Regular error type for CAM-related actions. In case of emergency, just do
///
/// ```
/// Err(CAMError::current())?
/// ```
#[derive(Debug)]
pub struct CAMError(String);

impl CAMError {
	pub fn current() -> Self { CAMError(
		unsafe {
			CStr::from_ptr(
				// strdup() to avoid implicit deallocation of external static variable
				libc::strdup(bindings::cam_errbuf.as_ptr())
			).to_string_lossy().into_owned()
		}
	) }
	pub fn from_status(dev: &CAMDevice, ccb: &CCB) -> Self {
		// the same comments about with_capacity() as in scsi/linux's SCSIDevice::do_cmd() apply here
		let mut s = vec![0; 512];

		unsafe {
			let err = bindings::cam_error_string(
				dev.0, ccb.0,
				s.as_mut_ptr(), s.capacity() as i32,
				bindings::cam_error_string_flags::CAM_ESF_ALL,
				bindings::cam_error_proto_flags::CAM_EPF_ALL,
			);

			CAMError(CStr::from_ptr(err).to_string_lossy().into_owned())
		}
	}
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
impl From<CAMError> for io::Error {
	fn from(err: CAMError) -> Self {
		io::Error::new(io::ErrorKind::Other, err)
	}
}
