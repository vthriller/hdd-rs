//! Functions implementing typical log page queries

use Device;
use super::SCSIDevice;
use super::data::log_page;

extern crate byteorder;
use byteorder::{ReadBytesExt, BigEndian};

use std::collections::HashMap;
use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ErrorCounter {
	/// Errors corrected without substantial delay
	CorrectedNoDelay,
	/// Errors corrected with possible delays
	CorrectedDelay,
	/// Total (e.g., rewrites or rereads)
	Total, // XXX total what?
	/// Total errors corrected
	ErrorsCorrected,
	/// Total times correction algorithm processed
	CRCProcessed,
	/// Total bytes processed
	BytesProcessed,
	/// Total uncorrected errors
	Uncorrected,
	VendorSpecific(u16),
	Reserved(u16),
}

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

	/**
	Asks for log page `page` and interprets its contents as a list of error counters

	Use the following instead:

	* [write_error_counters](#method.write_error_counters)
	* [read_error_counters](#method.read_error_counters)
	* [read_reverse_error_counters](#method.read_reverse_error_counters)
	* [verify_error_counters](#method.verify_error_counters)
	*/
	fn error_counters(&self, page: u8) -> Result<HashMap<ErrorCounter, u64>, Error> {
		let (_sense, data) = self.log_sense(
			false, // changed
			false, // save_params
			false, // default
			false, // threshold
			page, 0, // page, subpage
			0, // param_ptr
		)?;

		let page = log_page::parse(&data).ok_or(Error::new(ErrorKind::Other, "Unable to parse log page data"))?;
		let params = page.parse_params().ok_or(Error::new(ErrorKind::Other, "Unable to parse log page params"))?;

		let counters = params.iter().map(|param| {
			// XXX tell about unexpected params?
			if param.value.len() == 0 { return None; }

			let counter = match param.code {
				0x0000 => ErrorCounter::CorrectedNoDelay,
				0x0001 => ErrorCounter::CorrectedDelay,
				0x0002 => ErrorCounter::Total,
				0x0003 => ErrorCounter::ErrorsCorrected,
				0x0004 => ErrorCounter::CRCProcessed,
				0x0005 => ErrorCounter::BytesProcessed,
				0x0006 => ErrorCounter::Uncorrected,
				x @ 0x8000...0xffff => ErrorCounter::VendorSpecific(x),
				x => ErrorCounter::Reserved(x),
			};
			let value = (&param.value[..]).read_uint::<BigEndian>(param.value.len()).unwrap();

			Some((counter, value))
		})
		.filter(|kv| kv.is_some())
		.map(|kv| kv.unwrap())
		.collect();

		Ok(counters)
	}

	fn write_error_counters(&self) -> Result<HashMap<ErrorCounter, u64>, Error> {
		self.error_counters(0x02)
	}
	fn read_error_counters(&self) -> Result<HashMap<ErrorCounter, u64>, Error> {
		self.error_counters(0x03)
	}
	fn read_reverse_error_counters(&self) -> Result<HashMap<ErrorCounter, u64>, Error> {
		self.error_counters(0x04)
	}
	fn verify_error_counters(&self) -> Result<HashMap<ErrorCounter, u64>, Error> {
		self.error_counters(0x05)
	}
}

impl Pages for Device {}
