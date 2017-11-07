/*!
Functions to parse and structs to represent SCSI log pages.

For more, see SPC-4, 7.3 Log parameters.

## Example

```
use hdd::scsi::data::log_page;

let (_sense, data) = dev.log_sense(...)?;

let page = log_page::parse(&data).unwrap();
println!("{:#?}", page);

let params = page.parse_params();
for param in params {
	println!("{:#?}", param);
}
```
*/

use byteorder::{ReadBytesExt, BigEndian};

/**
When devices server should establish a unit attention condition (SAM-4).

Makes no sense for parameters of format other than `Format::BoundedCounter`.
*/
#[derive(Debug)]
pub enum Condition {
	/// Values are never compared.
	Never,
	/// Condition is always true.
	Always,
	/// Cumulative = Threshold.
	Eq,
	/// Cumulative ≠ Threshold.
	Ne,
	/// Cumulative > Threshold.
	Gt,
}

#[derive(Debug)]
pub enum Format { BoundedCounter, UnboundedCounter, ASCIIList, BinaryList }

#[derive(Debug)]
pub struct Parameter {
	pub code: u16,
	/// Whether cumulative parameter reflects all the events, or is only updated by the LOG SELECT command. For threshold parameters, this should be `false`.
	pub update_disabled: bool,
	/// Whether this parameter is implicitly saved at vendor-specific intervals; inverse of the TCD bit. (See also Control Mode Page bit GLTSD.)
	pub target_save: bool,
	/// See [enum Condition](enum.Condition.html)
	pub threshold_comparison: Condition,
	pub format: Format,
	pub value: Vec<u8>,
}

#[derive(Debug)]
pub struct Page {
	pub page: u8,
	pub subpage: Option<u8>,
	/// Whether this paged is saved if LOG SENSE is executed with SP bit set; inverse of DS log page bit
	pub saved: bool,
	pub data: Vec<u8>,
}

impl Page {
	// TODO? as iterator (but then, how to deal with invalid pages?)
	/**
	Parse page data as list of params.

	Note that not all pages contain params; page 00h (Supported Log Pages) is a notable example, as it represents list of supported pages with a simple array of `u8`s.

	Returns `None` if some param spans past the transferred data buffer (usually it means that it's not the params that are attached to the page).
	*/
	pub fn parse_params(&self) -> Option<Vec<Parameter>> {
		let mut params = vec![];

		// iterate over params
		let mut current_param: usize = 0;
		let len = self.data.len();
		while current_param < len {
			if current_param + 4 > len {
				return None; // not enough data
			}

			let code = (&self.data[current_param .. current_param + 2]).read_u16::<BigEndian>().unwrap();
			let control = self.data[current_param + 2];
			let plen = self.data[current_param + 3] as usize;

			// skip this param's header
			current_param += 4;

			if current_param + plen > len {
				return None; // not enough data
			}

			params.push(Parameter {
				code: code,

				update_disabled: control & 0b10000000 != 0,
				target_save: control & 0b100000 != 0,
				threshold_comparison: {
					use self::Condition::*;
					match (control & 0b10000 != 0, (control & 0b1100) >> 2) {
						(false, _) => Never,
						(true, 0b00) => Always,
						(true, 0b01) => Eq,
						(true, 0b10) => Ne,
						(true, 0b11) => Gt,
						_ => unreachable!(),
					}
				},
				format: match control & 0b11 {
					0b00 => Format::BoundedCounter,
					0b01 => Format::ASCIIList,
					0b10 => Format::UnboundedCounter,
					0b11 => Format::BinaryList,
					_ => unreachable!(),
				},
				value: self.data[current_param .. current_param+plen].to_vec(),
			});

			current_param += plen;
		}

		Some(params)
	}
}

// TODO return Result<>
pub fn parse(data: &[u8]) -> Option<Page> {
	if data.len() < 4 {
		return None;
	}

	// data[2..4] is Page Length, starting from data[4],
	let len = ((&data[2..4]).read_u16::<BigEndian>().unwrap() + 4) as usize;

	if data.len() < len {
		// not enough data
		return None;
	}

	Some(Page {
		saved: data[0] & 0b10000000 == 0,
		page: data[0] & 0b111111,
		subpage: match (data[0] & 0b1000000 != 0, data[1]) {
			(false, 0) => None,
			// we're not expecting subpage != 0 if SPF bit is unset
			(false, _) => { return None },
			(true, sp) => Some(sp),
		},
		data: data[4 .. len].to_vec(),
	})
}
