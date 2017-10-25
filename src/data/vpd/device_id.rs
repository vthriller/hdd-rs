#[derive(Debug)]
pub enum Protocol {
	None,
	FC, // Fibre Channel, FCP-2
	SCSI, // Parallel SCSI, SPI-4
	SSA, // SSA-S3P
	FireWire, // IEEE 1394, SBP-2
	RDMA, // SRP
	ISCSI,
	SAS,
	Reserved(u8),
}

#[derive(PartialEq, Debug)]
pub enum CodeSet {
	Binary,
	ASCII, // 0x20 through 0x7e
	Reserved(u8),
}

#[derive(PartialEq, Debug)]
pub enum Association {
	Device, // addressed physical or logical device
	Port, // port that received the request
	Target, // SCSI target device that contains the addressed logical unit
	Reserved,
}

#[derive(Debug)]
pub enum Identifier<'a> {
	VendorSpecific(&'a [u8]),
	// TODO? [u8; 8] for vendor_id
	Generic { vendor_id: &'a [u8], id: &'a [u8] },
	FCNameIdentifier(&'a [u8]), // FC-PH/FC-PH3/FC-FS Name_Identifier
	// TODO? [u8; 8]
	EUI64(&'a [u8]), // IEEE Extended Unique Identifier
	Port(u32),
	MD5(&'a [u8]),
	Reserved(u8),
	Invalid,
}

#[derive(Debug)]
pub struct Descriptor<'a> {
	pub proto: Protocol,
	pub codeset: CodeSet,
	pub assoc: Association,
	pub id: Identifier<'a>,
}

pub fn parse(data: &[u8]) -> Vec<Descriptor> {
	let mut descriptors = vec![];

	let mut i = 0;
	while i < data.len() {
		let idlen = data[i+3] as usize;
		let id = &data[i .. i + idlen + 4];

		let proto = if id[1] & 0b10000000 == 0 {
			Protocol::None // Protocol Identifier Valid bit is not set, Protocol Identifier must be ignored
		} else {
			match id[0] >> 4 {
				0 => Protocol::FC,
				1 => Protocol::SCSI,
				2 => Protocol::SSA,
				3 => Protocol::FireWire,
				4 => Protocol::RDMA,
				5 => Protocol::ISCSI,
				6 => Protocol::SAS,
				x => Protocol::Reserved(x),
			}
		};

		let codeset = match id[0] & 0b1111 {
			// 0 is also reserved
			1 => CodeSet::Binary,
			2 => CodeSet::ASCII,
			x => CodeSet::Reserved(x),
		};

		let assoc = match (id[1] >> 4) & 0b11 {
			0 => Association::Device,
			1 => Association::Port,
			2 => Association::Target,
			3 => Association::Reserved,
			_ => unreachable!(),
		};

		let id = match id[1] & 0b1111 { // match by identifier type
			0 => Identifier::VendorSpecific(&id[4..]),
			1 => Identifier::Generic {
				vendor_id: &id[4..12],
				id: &id[12..],
			},
			2 => Identifier::EUI64(&id[4..]),
			3 => Identifier::FCNameIdentifier(&id[4..]),
			x@4 | x@5 => if assoc == Association::Port {
				if !(codeset == CodeSet::Binary && idlen == 4) { Identifier::Invalid }
				else {
					Identifier::Port(
						((id[4] as u32) << 24) +
						((id[5] as u32) << 16) +
						((id[6] as u32) << 8) +
						((id[7] as u32))
					)
				}
			} else {
				Identifier::Reserved(x)
			},
			6 => if assoc == Association::Device {
				if !(codeset == CodeSet::Binary && idlen == 4) { Identifier::Invalid }
				else {
					Identifier::Port(
						((id[4] as u32) << 24) +
						((id[5] as u32) << 16) +
						((id[6] as u32) << 8) +
						((id[7] as u32))
					)
				}
			} else {
				Identifier::Reserved(6)
			},
			7 => Identifier::MD5(&id[4..]),
			x => Identifier::Reserved(x),
		};

		descriptors.push(Descriptor {
			proto: proto,
			codeset: codeset,
			assoc: assoc,
			id: id,
		});

		i += 4 + idlen;
	}
	descriptors
}
