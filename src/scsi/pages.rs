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

#[derive(Debug)]
pub struct Date {
	// TODO u8, u16
	pub week: String,
	pub year: String,
}

#[derive(Debug)]
pub struct DatesAndCycleCounters {
	pub manufacturing_date:	Option<Date>,
	/// Date in which the device was placed in service
	pub accounting_date:	Option<Date>,
	pub lifetime_start_stop_cycles:	Option<u32>,
	pub start_stop_cycles:	Option<u32>,
	pub lifetime_load_unload_cycles:	Option<u32>,
	pub load_unload_cycles:	Option<u32>,
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

	fn non_medium_error_count(&self) -> Result<u64, Error> {
		let (_sense, data) = self.log_sense(
			false, // changed
			false, // save_params
			false, // default
			false, // threshold
			0x06, 0, // page, subpage
			0, // param_ptr
		)?;

		let page = log_page::parse(&data).ok_or(Error::new(ErrorKind::Other, "Unable to parse log page data"))?;
		let params = page.parse_params().ok_or(Error::new(ErrorKind::Other, "Unable to parse log page params"))?;

		for param in params {
			// XXX tell about unexpected params?
			if param.value.len() == 0 { continue; }
			if param.code != 0 { continue; }

			return Ok((&param.value[..]).read_uint::<BigEndian>(param.value.len()).unwrap());
		}

		Err(Error::new(ErrorKind::Other, "Cannot find valid param for this log page"))
	}

	/**
	Returns tuple of `(temp, ref_temp)`, where:

	* `temp`: current temperature, °C,
	* `ref_temp`: reference temperature, °C; maximum temperature at which device is capable of operating continuously without degrading
	*/
	fn temperature(&self) -> Result<(Option<u8>, Option<u8>), Error> {
		let (_sense, data) = self.log_sense(
			false, // changed
			false, // save_params
			false, // default
			false, // threshold
			0x0d, 0, // page, subpage
			0, // param_ptr
		)?;

		let page = log_page::parse(&data).ok_or(Error::new(ErrorKind::Other, "Unable to parse log page data"))?;
		let params = page.parse_params().ok_or(Error::new(ErrorKind::Other, "Unable to parse log page params"))?;

		let mut temp = None;
		let mut ref_temp = None;

		for param in params {
			// XXX tell about unexpected params?
			if param.value.len() < 2 { continue; }

			// value[0] is reserved
			let value = match param.value[1] {
				0xff => None, // unable to return temperature despite including this param in the answer
				x => Some(x),
			};

			match param.code {
				0x0000 => { temp = value },
				0x0001 => { ref_temp = value },
				_ => (),
			};
		}

		Ok((temp, ref_temp))
	}

	/// In SPC-4, this is called Start-Stop Cycle Counter
	fn dates_and_cycle_counters(&self) -> Result<DatesAndCycleCounters, Error> {
		let (_sense, data) = self.log_sense(
			false, // changed
			false, // save_params
			false, // default
			false, // threshold
			0x0e, 0, // page, subpage
			0, // param_ptr
		)?;

		let page = log_page::parse(&data).ok_or(Error::new(ErrorKind::Other, "Unable to parse log page data"))?;
		let params = page.parse_params().ok_or(Error::new(ErrorKind::Other, "Unable to parse log page params"))?;

		let mut result = DatesAndCycleCounters {
			manufacturing_date: None,
			accounting_date: None,
			lifetime_start_stop_cycles: None,
			start_stop_cycles: None,
			lifetime_load_unload_cycles: None,
			load_unload_cycles: None,
		};

		for param in params {
			match param.code {
				0x0001 => {
					// XXX tell about unexpected params?
					if param.value.len() < 6 { continue; }

					result.manufacturing_date = Some(Date {
						year: String::from_utf8(param.value[0..4].to_vec()).unwrap(), // ASCII
						week: String::from_utf8(param.value[4..6].to_vec()).unwrap(), // ASCII
					});
				},
				0x0002 => {
					// XXX tell about unexpected params?
					if param.value.len() < 6 { continue; }

					result.accounting_date = Some(Date {
						year: String::from_utf8(param.value[0..4].to_vec()).unwrap(), // ASCII, might be all-spaces
						week: String::from_utf8(param.value[4..6].to_vec()).unwrap(), // ASCII, might be all-spaces
					});
				},
				0x0003 => {
					// XXX tell about unexpected params?
					if param.value.len() < 4 { continue; }

					result.lifetime_start_stop_cycles = Some(
						(&param.value[0 .. 4]).read_u32::<BigEndian>().unwrap()
					);
				},
				0x0004 => {
					// XXX tell about unexpected params?
					if param.value.len() < 4 { continue; }

					result.start_stop_cycles = Some(
						(&param.value[0 .. 4]).read_u32::<BigEndian>().unwrap()
					);
				},
				0x0005 => {
					// XXX tell about unexpected params?
					if param.value.len() < 4 { continue; }

					result.lifetime_load_unload_cycles = Some(
						(&param.value[0 .. 4]).read_u32::<BigEndian>().unwrap()
					);
				},
				0x0006 => {
					// XXX tell about unexpected params?
					if param.value.len() < 4 { continue; }

					result.load_unload_cycles = Some(
						(&param.value[0 .. 4]).read_u32::<BigEndian>().unwrap()
					);
				},
				_ => {
					// XXX tell about unexpected params?
				},
			}
		}

		Ok(result)
	}
}

impl Pages for Device {}
