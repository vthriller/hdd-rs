//! Functions implementing typical log page queries

use Device;
use super::SCSIDevice;
use super::data::log_page;

use std::io::{Error, ErrorKind};

// TODO proper errors
// TODO non-empty autosense errors
/// Methods in this trait issue LOG SENSE command against the device and return interpreted log page responses
pub trait Pages: SCSIDevice {
	// TODO? use this in a constructor of a new type to prevent user from issuing LOG SENSE against unsupported log pages
	fn supported_pages(&self) -> Result<Vec<u8>, Error> {
		let (_sense, data) = self.log_sense(
			false, // changed
			false, // save_params
			false, // default
			false, // threshold
			0, 0, // page, subpage
			0, // param_ptr
		)?;

		log_page::parse(&data).map(|page| {
			page.data.to_vec()
		}).ok_or(Error::new(ErrorKind::Other, "Unable to parse log page data"))
	}

}

impl Pages for Device {}
