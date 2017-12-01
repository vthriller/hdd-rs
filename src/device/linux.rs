use std::fs::File;
use std::io;

/// See [parent module docs](../index.html)
#[derive(Debug)]
pub struct Device {
	pub file: File,
}

#[derive(Debug)]
pub enum Type { SCSI }

impl Device {
	pub fn open(path: &str) -> Result<Self, io::Error> {
		Ok(Device {
			file: File::open(path)?,
		})
	}

	pub fn get_type(&self) -> Result<Type, io::Error> { Ok(Type::SCSI) }
}
