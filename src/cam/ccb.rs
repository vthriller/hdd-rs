use cam::bindings;
use cam::device::CAMDevice;

use std::mem;

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
	pub fn new(dev: &CAMDevice) -> Self {
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
		unsafe { bindings::cam_freeccb(self.0); }
	}
}
