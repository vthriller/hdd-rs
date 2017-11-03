mod fixed;
pub use self::fixed::FixedData;

mod descriptor;
pub use self::descriptor::{Descriptor, DescriptorData};

#[derive(Debug)]
pub enum Sense<'a> {
	Fixed(FixedData<'a>),
	Descriptor(DescriptorData<'a>),
}

/**
Parses sense data of any of the supported formats (70h–73h).

Returns tuple `(current, data)`, where `current` indicates whether this sense represents current or deferred error; or `None` if:

* format is not recognized,
* `data` buffer has not enough data to decode sense.

## Panics

Panics if `data` is empty.
*/
pub fn parse(data: &[u8]) -> Option<(bool, Sense)> {
	let response_code = data[0] & 0x7f;
	let (fixed, current) = match response_code {
		0x70 => (true, true),
		0x71 => (true, false),
		0x72 => (false, true),
		0x73 => (false, false),
		_ => return None,
	};

	let data = if fixed {
		fixed::parse(data).map(|data| Sense::Fixed(data))
	} else {
		descriptor::parse(data).map(|data| Sense::Descriptor(data))
	};

	data.map(|data| (current, data))
}
