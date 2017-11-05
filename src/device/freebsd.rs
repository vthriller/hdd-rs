use cam::{CAMDevice, CAMError};

/// See [parent module docs](../index.html)
#[derive(Debug)]
pub struct Device {
	pub dev: CAMDevice,
}

impl Device {
	pub fn open(path: &str) -> Result<Self, CAMError> {
		Ok(Device {
			dev: CAMDevice::open(path)?,
		})
	}
}
