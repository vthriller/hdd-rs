#[derive(Debug)]
pub struct Descriptor<'a> {
	/// Descriptor Type
	pub code: u8,
	/// Descriptor Specific Data
	pub data: &'a [u8],
}

#[derive(Debug)]
pub struct DescriptorData<'a> {
	pub key: u8,
	/// Additional Sense Code
	pub asc: u8,
	/// Additional Sense Code Qualifier
	pub ascq: u8,
	pub descriptors: Vec<Descriptor<'a>>,
}

pub fn parse(data: &[u8]) -> Option<DescriptorData> {
	if data.len() < 8 {
		return None;
	}

	// data[7] is Additional Sense Length, starting from data[8],
	let len = (data[7] + 8) as usize;
	let mut descriptors = vec![];

	if data.len() < len {
		// not enough data
		return None;
	}

	// iterate over descriptors
	let mut current_desc: usize = 8;
	while current_desc < len {
		let (code, dlen) = (data[current_desc], data[current_desc + 1]);
		let dlen = dlen as usize;

		// skip this descriptors' header
		current_desc += 2;

		descriptors.push(Descriptor {
			code: code,
			data: &data[current_desc .. current_desc+dlen],
		});

		current_desc += dlen;
	}

	Some(DescriptorData {
		key: data[1] & 0b1111,
		asc: data[2],
		ascq: data[3],
		descriptors: descriptors,
	})
}
