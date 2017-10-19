extern crate libc;
use std::ffi::{CStr, CString};
use std::mem;

use std::error;
use std::fmt;

extern crate cam;

use ata;

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


pub fn ata_exec(file: &str, cmd: ata::Command, sector: u8, feature: u8, nsector: u8) -> Result<[u8; 512], CAMError> {
	let dev = CAMDevice::open(file)?;

	let timeout = 10; // in seconds; TODO configurable

	let mut data: [u8; 512] = [0; 512];

	let mut ccb: cam::ccb = unsafe { mem::zeroed() };

	unsafe {
		let h = ccb.ccb_h.as_mut();
		h.func_code = cam::xpt_opcode::XPT_ATA_IO;
		h.flags = cam::ccb_flags::CAM_DIR_IN as u32;
		h.retry_count = 0;
		h.timeout = timeout * 1000;

		let ataio = ccb.ataio.as_mut();
		ataio.data_ptr = data.as_mut_ptr();
		ataio.dxfer_len = 512;
		ataio.ata_flags = 0;

		ataio.cmd.command = cmd as u8;
		ataio.cmd.features = feature;
		ataio.cmd.lba_low = sector;
		ataio.cmd.lba_mid = 0;
		ataio.cmd.lba_high = 0;
		ataio.cmd.lba_low_exp = 0;
		ataio.cmd.lba_mid_exp = 0;
		ataio.cmd.lba_high_exp = 0;
		ataio.cmd.device = 0x40;
		ataio.cmd.sector_count = 1;

		ataio.cmd.flags = (cam::CAM_ATAIO_NEEDRESULT | cam::CAM_ATAIO_48BIT) as u8;

		h.flags |= cam::ccb_flags::CAM_DEV_QFRZDIS as u32;
	}

	if unsafe { cam::cam_send_ccb(dev.0, &mut ccb) } < 0 {
		return Err(CAMError::current())
	}

	// XXX what.
	if (unsafe { ccb.ccb_h.as_ref() }.status & (cam::cam_status_CAM_STATUS_MASK as u32)) != (cam::cam_status::CAM_REQ_CMP as u32) {
		return Err(CAMError::current())
	}

	Ok(data)
}
