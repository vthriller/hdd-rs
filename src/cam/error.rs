use cam::bindings;
use cam::device::CAMDevice;
use cam::ccb::CCB;

use std::ffi::CStr;
use std::io;

extern crate libc;

	pub fn current() -> io::Error { io::Error::new(io::ErrorKind::Other,
		unsafe {
			CStr::from_ptr(
				// strdup() to avoid implicit deallocation of external static variable
				libc::strdup(bindings::cam_errbuf.as_ptr())
			).to_string_lossy().into_owned()
		}
	) }
	pub fn from_status(dev: &CAMDevice, ccb: &CCB) -> io::Error {
		// the same comments about with_capacity() as in scsi/linux's SCSIDevice::do_cmd() apply here
		let mut s = vec![0; 512];

		unsafe {
			let err = bindings::cam_error_string(
				dev.0, ccb.0,
				s.as_mut_ptr(), s.capacity() as i32,
				bindings::cam_error_string_flags::CAM_ESF_ALL,
				bindings::cam_error_proto_flags::CAM_EPF_ALL,
			);

			io::Error::new(io::ErrorKind::Other,
				CStr::from_ptr(err).to_string_lossy().into_owned()
			)
		}
	}
