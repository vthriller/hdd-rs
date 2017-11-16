use std::fs::File;
use std::io;

/// See [parent module docs](../index.html)
#[derive(Debug)]
pub struct Device {
	pub file: File,
}

impl Device {
	pub fn open(path: &str) -> Result<Self, io::Error> {
		Ok(Device {
			file: File::open(path)?,
		})
	}
}
