mod fixed;
pub use self::fixed::FixedData;

#[derive(Debug)]
pub enum Sense<'a> {
	Fixed(FixedData<'a>),
	Descriptor, // TODO
}

/**
Parses sense data of any of the supported formats (70h–73h).

Returns tuple `(deferred, data)`, where `deferred` indicates whether this sense represents current or deferred error; or `None` if:

* format is not recognized,
* `data` buffer has not enough data to decode sense.

## Panics

Panics if `data` is empty.
*/
pub fn parse(data: &[u8]) -> Option<(bool, Sense)> {
	let response_code = data[0] & 0x7f;
	let (fixed, deferred) = match response_code {
		0x70 => (true, false),
		0x71 => (true, true),
		0x72 => (false, false),
		0x73 => (false, true),
		_ => return None,
	};

	let data = if fixed {
		fixed::parse(data).map(|data| Sense::Fixed(data))
	} else {
		Some(Sense::Descriptor)
	};

	data.map(|data| (deferred, data))
}
