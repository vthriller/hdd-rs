#[derive(Serialize, Debug)]
pub struct Inquiry {
	connected: Option<bool>,
	device_type: String, // TODO enum?

	removable: bool,

	naca_bit: bool, /// Normal ACA bit support, see SAM-3
	hier_addressing: bool,

	scc:  bool,
	acc:  bool,
	tpc:  bool,
	protection:  bool,

	enclosure_services: bool,
	multiport: bool,
	media_changer: bool,
	linked_cmds: bool,

	vendor_id: String,
	product_id: String,
	product_rev: String,
	drive_serial: String,
}

fn is_set(x: u8, bit: usize) -> bool {
	x & (1<<bit) != 0
}

pub fn parse_inquiry(data: &Vec<u8>) -> Inquiry {
	Inquiry {
		connected: match (data[0] & 0b11100000) >> 5 { // Peripheral Qualifier
			0b000 => Some(true),
			0b001 => Some(false),
			// 010 is reserved
			// 011 for server not capable of supporting a peripheral device
			// 100..111 is vendor specific
			_ => None,
		},
		device_type: match data[0] & 0b00011111 {
			0x00 => "SBC-2", // Direct access block device (e.g., magnetic disk)
			0x01 => "SSC-2", // Sequential-access device (e.g., magnetic tape)
			0x02 => "SSC", // Printer device
			0x03 => "SPC-2", // Processor device
			0x04 => "SBC", // Write-once device (e.g., some optical disks)
			0x05 => "MMC-4", // CD/DVD device
			0x06 => "Scanner device", // (obsolete)
			0x07 => "SBC", // Optical memory device (e.g., some optical disks)
			0x08 => "SMC-2", // Medium changer device (e.g., jukeboxes)
			0x09 => "Communications device", // (obsolete)
			0x0A | 0x0B => "?", // Obsolete
			0x0C => "SCC-2", // Storage array controller device (e.g., RAID)
			0x0D => "SES", // Enclosure services device
			0x0E => "RBC", // Simplified direct-access device (e.g., magnetic disk)
			0x0F => "OCRW", // Optical card reader/writer device
			0x10 => "BCC", // Bridge Controller Commands
			0x11 => "OSD", // Object-based Storage Device
			0x12 => "ADC", // Automation/Drive Interface
			0x13 | 0x1D => "Reserved",
			0x1E => "Well known logical unit",
			0x1F => "Unknown or no device type",
			_ => unreachable!(),
		}.to_string(),

		removable: is_set(data[1], 7),

		/* TODO data[2] → Version:
		0x03 SPC
		0x04 SPC-2
		the rest is ???
		*/

		naca_bit: is_set(data[3], 5),
		hier_addressing: is_set(data[3], 4),

		// ResponseFormat: data[3] & 0b1111, // TODO
		// data[4]: additional length

		scc: is_set(data[5], 7), // storage array controller component support
		acc: is_set(data[5], 6), // device contains an access controls coordinator
		// TODO? (data[5] & 0b00110000) Target Port Group Support
		tpc: is_set(data[5], 3), // support for 3rd-party copy commands
		protection: is_set(data[5], 0),

		enclosure_services: is_set(data[6], 6),
		multiport: is_set(data[6], 4),
		media_changer: is_set(data[6], 3),
		linked_cmds: is_set(data[7], 3),

		/* TODO? match (bque: is_set(data[6], 7), cmdque: is_set(data[7], 1)):
		00 obsolete
		01 full task mgmt model
		01 basic task mgmt model
		11 invalid
		*/

		// XXX? > ASCII data fields … may be terminated with one or more ASCII null (00h) characters.
		vendor_id: String::from_utf8(data[8..16].to_vec()).unwrap().trim().to_string(),
		product_id: String::from_utf8(data[16..32].to_vec()).unwrap().trim().to_string(),
		product_rev: String::from_utf8(data[32..36].to_vec()).unwrap().trim().to_string(),
		drive_serial: String::from_utf8(data[36..44].to_vec()).unwrap().trim().to_string(),

		// TODO TODO TODO TODO TODO
	}
}
