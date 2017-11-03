use std::fs::File;
use std::io::Error;

/// See [parent module docs](../index.html)
pub struct Device {
	pub file: File,
}

impl Device {
	pub fn open(path: &str) -> Result<Self, Error> {
		Ok(Device {
			file: File::open(path)?,
		})
	}
}
