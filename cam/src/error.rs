use bindings;

use std::ffi::CStr;
use std::error;
use std::io::{Error, ErrorKind};
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
