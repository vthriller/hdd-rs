use cam::bindings;
use cam::device::CAMDevice;
use cam::ccb::CCB;

use std::ffi::CStr;
use std::io;

use libc;

/**
Returns errors reported by some of the functions described in cam(3).

Use this function if the following ones returned NULL:

- cam_open_device
- cam_open_spec_device
- cam_open_btl
- cam_open_pass
- cam_getccb
- cam_device_dup

Use this function if the following ones returned -1:

- cam_get_device

**Note:** cam_send_ccb is a simple `return ioctl(…)` function and thus its errors are *not* rendered in `cam_errbuf`.
*/
pub fn current() -> io::Error { io::Error::new(io::ErrorKind::Other,
	unsafe {
		CStr::from_ptr(
			// strdup() to avoid implicit deallocation of external static variable
			libc::strdup(bindings::cam_errbuf.as_ptr())
		).to_string_lossy().into_owned()
	}
) }

/// Returns errors indicated with `ccb.ccb.h.status & CAM_STATUS_MASK`.
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
