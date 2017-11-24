use cam::CAMDevice;
use std::io;

/// See [parent module docs](../index.html)
#[derive(Debug)]
pub struct Device {
	pub dev: CAMDevice,
}

impl Device {
	pub fn open(path: &str) -> Result<Self, io::Error> {
		Ok(Device {
			dev: CAMDevice::open(path)?,
		})
	}
}
