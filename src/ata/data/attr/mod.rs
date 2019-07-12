pub mod raw;

#[cfg(feature = "drivedb-parser")]
use std::collections::HashMap;
#[cfg(feature = "drivedb-parser")]
use drivedb;

#[derive(Debug)]
#[cfg_attr(feature = "serializable", derive(Serialize))]
pub struct SmartAttribute {
	pub id: u8,

	// XXX make sure it's exactly 12 bytes
	// FIXME extra allocations
	#[cfg_attr(feature = "serializable", serde(skip_serializing))]
	_data: Vec<u8>,

	pub name: Option<String>, // comes from the drivedb

	// contains None if `raw` is rendered using byte that usually covers this value
	// TODO? 0x00 | 0xfe | 0xff are invalid
	pub value: Option<u8>,
	// contains None if `raw` is rendered using byte that usually covers this value
	pub worst: Option<u8>,

	#[cfg_attr(feature = "serializable", serde(skip_serializing))]
	_attr_meta: Option<drivedb::vendor_attribute::Attribute>,

	pub thresh: Option<u8>, // requested separately; TODO? 0x00 is "always passing", 0xff is "always failing", 0xfe is invalid
}

impl SmartAttribute {
	#[cfg(feature = "drivedb-parser")]
	pub fn raw(&self) -> raw::Raw {
		raw::Raw::from_raw_entry(&self._data, &self._attr_meta)
	}

	#[inline]
	fn flags(&self) -> u16 {
		(self._data[1] as u16) + ((self._data[2] as u16) << 8) // XXX endianness?
	}

	// if true, failure is predicted within 24h; otherwise, attribute indicates drive's exceeded intended design life period
	pub fn pre_fail(&self)        -> bool { self.flags() & (1<<0) != 0 }
	pub fn online(&self)          -> bool { self.flags() & (1<<1) != 0 }
	// In SFF-8035i rev 2, bits 2-5 are defined as vendor-specific, and 6-15 are reserved;
	// however, these days the following seems to be universally interpreted the way it was once (probably) established by IBM, Maxtor and Quantum
	pub fn performance(&self)     -> bool { self.flags() & (1<<2) != 0 }
	pub fn error_rate(&self)      -> bool { self.flags() & (1<<3) != 0 }
	pub fn event_count(&self)     -> bool { self.flags() & (1<<4) != 0 }
	pub fn self_preserving(&self) -> bool { self.flags() & (1<<5) != 0 }
	pub fn misc_flags(&self)      -> u16  { self.flags() & (!0b11_1111) }
}

#[cfg(feature = "drivedb-parser")]
fn parse_thresholds(raw: &[u8]) -> HashMap<u8, u8> {
	let mut threshs = HashMap::<u8, u8>::new();

	// skip (XXX check?) data struct revision number
	let raw = &raw[2..];

	// there are 30 entries, each 12-byte wide
	// TODO chunks_exact (rust >= 1.31)
	let raw = raw.chunks(12).take(30);

	for entry in raw {
		let attr = entry[0];
		let thresh = entry[1];
		// fields 2..11 are reserved

		// attribute table entry of id 0x0 is invalid
		if attr == 0 { continue }

		threshs.insert(attr, thresh);
	}
	threshs
}

fn is_in_raw(attr: &Option<drivedb::vendor_attribute::Attribute>, c: char) -> bool {
	attr.as_ref().map(|a| a.byte_order.contains(c)).unwrap_or(false)
}

#[cfg(feature = "drivedb-parser")]
pub fn parse_smart_values(data: &[u8], raw_thresh: &[u8], meta: &Option<drivedb::DriveMeta>) -> Vec<SmartAttribute> {
	// TODO cover bytes 0..1 362..511 of data
	// XXX what if some drive reports the same attribute multiple times?
	// TODO return None if data.len() < 512

	let threshs = parse_thresholds(raw_thresh);

	// skip (XXX check?) data struct revision number
	let data = &data[2..];

	// there are 30 entries, each 12-byte wide
	// TODO chunks_exact (rust >= 1.31)
	let data = data.chunks(12).take(30);

	let mut attrs = vec![];
	for entry in data {
		let id = entry[0];

		// attribute table entry of id 0x0 is invalid
		if id == 0 { continue }

		let attr = meta.as_ref().map(|meta| meta.render_attribute(id)).unwrap_or(None);

		attrs.push(SmartAttribute {
			id: id,

			_data: entry.to_vec(),

			name: match &attr {
				Some(a) => a.name.clone(),
				None => None
			},

			value: if !is_in_raw(&attr, 'v') {
				Some(entry[3])
			} else { None },
			worst: if !is_in_raw(&attr, 'w') {
				Some(entry[4])
			} else { None },

			_attr_meta: attr,

			// .get() returns Option<&T>, but threshs would not live long enough, and it's just easier to copy u8 using this map
			thresh: threshs.get(&id).map(|&t| t),
		})
	}
	attrs
}
