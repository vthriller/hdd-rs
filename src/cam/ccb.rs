use cam::bindings;
use cam::device::CAMDevice;

use std::mem;

/// Thin wrapper for `union ccb *`, the CAM Control Block. Note that the underlying raw pointer is *mutable*.
///
/// This struct implements `Drop`, i.e. you don't need to call `cam_freeccb` yourself.
#[derive(Debug)]
pub struct CCB(pub *mut bindings::ccb);

impl CCB {
	/// Calls `cam_getccb` with provided device, and zeroes all the values except for those that compose its header (`ccb_h`), which is a common pratice (see smartmontools, camcontrol).
	///
	/// # Panics
	///
	/// This function panics if `cam_getccb` returns `NULL`, assuming there's not enough memory to allocate anything, not even `Err` to return.
	pub fn new(dev: &CAMDevice) -> Self {
		let ccb: *mut bindings::ccb = unsafe { bindings::cam_getccb(dev.0) };

		if ccb.is_null() {
			panic!("cannot allocate CCB");
		}

		let start = mem::size_of::<bindings::ccb_hdr>();
		let end = mem::size_of::<bindings::ccb>();
		unsafe {
			(ccb as *mut u8)
				.offset(start as isize)
				.write_bytes(0, end - start);
		}
		CCB(ccb)
	}

	pub fn get_status(&self) -> u32 {
		unsafe {
			(*self.0).ccb_h
		}.status & bindings::cam_status_CAM_STATUS_MASK
	}
	pub fn get_status_flags(&self) -> u32 {
		unsafe {
			(*self.0).ccb_h
		}.status
	}

	// those are deliberately kept unsafe
	pub unsafe fn ccb_h(&self) -> &mut bindings::ccb_hdr { &mut (*self.0).ccb_h }
	pub unsafe fn csio(&self) -> &mut bindings::ccb_scsiio { &mut (*self.0).csio }
	pub unsafe fn ataio(&self) -> &mut bindings::ccb_ataio { &mut (*self.0).ataio }
	pub unsafe fn cpi(&self) -> &mut bindings::ccb_pathinq { &mut (*self.0).cpi }
}

impl Drop for CCB {
	fn drop(&mut self) {
		unsafe { bindings::cam_freeccb(self.0); }
	}
}
