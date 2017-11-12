use std::fmt;

fn bytes_to_words(data: &Vec<u8>) -> Vec<u16> {
	let mut output = vec![];

	// XXX what if `data` contains odd number of u8s?
	for i in 0 .. data.len()/2 {
		if cfg!(target_endian = "little") {
			output.push(
				((data[2 * i + 1] as u16) << 8)
				+ (data[2 * i] as u16)
			);
		} else {
			output.push(
				((data[2 * i] as u16) << 8)
				+ (data[2 * i + 1] as u16)
			);
		}
	}

	output
}

// TODO make sure characters are in the range of 0x20 to (and including) 0x7e
// (this is in the standard, and also to make std::String safe again)
fn read_string(arr: &Vec<u16>, start: usize, fin: usize) -> String {
	let mut output = String::with_capacity((fin - start) * 2);

	for i in start..(fin+1) {
		output.push((arr[i] >> 8) as u8 as char);
		output.push((arr[i] & 0xff) as u8 as char);
	}

	String::from(output.trim())
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[cfg_attr(feature = "serializable", derive(Serialize))]
pub enum Ternary {
	Unsupported, Disabled, Enabled
}

impl fmt::Display for Ternary {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Ternary::Unsupported => write!(f, "not supported"),
			Ternary::Disabled    => write!(f, "supported, disabled"),
			Ternary::Enabled     => write!(f, "supported, enabled"),
		}
	}
}

#[derive(Debug)]
#[cfg_attr(feature = "serializable", derive(Serialize))]
pub enum RPM {
	Unknown, NonRotating, RPM(u16)
}

#[derive(Debug)]
#[cfg_attr(feature = "serializable", derive(Serialize))]
pub struct IdCommands {
	pub device_reset: bool,
	pub write_buffer: bool,
	pub read_buffer: bool,
	pub nop:  bool,
	pub download_microcode: bool,
	pub read_write_dma_queued: bool,
	pub flush_cache: bool,
	pub flush_cache_ext: bool,
	pub write_dma_fua_ext: bool, // also WRITE MULTIPLE FUA EXT
	pub write_dma_queued_fua_ext: bool,
	pub write_uncorrectable: bool,
	pub read_write_dma_ext_gpl: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "serializable", derive(Serialize))]
pub struct Id {
	pub is_ata: bool, // probably redundant
	pub incomplete: bool, // content of words other that 0 or 2 might be invalid

	pub serial: String,
	pub firmware: String,
	pub model: String,

	pub capacity: u64,
	pub sector_size_phy: u32,
	pub sector_size_log: u32,

	pub rpm: RPM,

	pub trusted_computing_supported: bool,

	pub ata_version: Option<&'static str>,

	pub commands_supported: IdCommands,

	pub power_mgmt_supported: bool,
	pub write_cache: Ternary,
	pub read_look_ahead: Ternary,
	pub hpa: Ternary, // Host Protected Area
	pub apm: Ternary, // Advanced Power Management
	pub aam: Ternary, // Automatic Acoustic Management
	pub gp_logging_supported: bool, // General Purpose Logging
	pub wwn_supported: bool, // World Wide Name
	pub security: Ternary,

	pub smart: Ternary,
	pub smart_error_logging_supported: bool,
	pub smart_self_test_supported: bool,
}

fn is_set(word: u16, bit: usize) -> bool {
	word & (1<<bit) != 0
}
fn make_ternary(data: &Vec<u16>, word_sup: usize, bit_sup: usize, word_enabled: usize, bit_enabled: usize) -> Ternary {
	if !is_set(data[word_sup], bit_sup) {
		Ternary::Unsupported
	} else {
		if is_set(data[word_enabled], bit_enabled) { Ternary::Enabled }
		else { Ternary::Disabled }
	}
}

pub fn parse_id(data: &Vec<u8>) -> Id {
	// TODO return None if data.len() < 512
	let data = bytes_to_words(data);
	/*
	TODO ATA8-ACS T13/1699-D Revision 3f field description
		vs Revision 6a
		vs crap knows what other revisions and standards

	TODO
	w0       if 0x848a, CFA feature set is supported, and `is_ata`, `incomplete` are irrelevant
	w2       values regarding incomplete id output
	w47      READ/WRITE MULTIPLE support
	w49:13   standby timer values: 1 if standard compliant, 0 if vendor-specific
	w49:11   IODRY is supported
	w49:10   IODRY can (1) or can not (0) be disabled
	w49:9    LBA transition is supported
	w49:8    DMA is supported
	w50:0    device has a minimum Standby timer value that is device-specific
	w59:8    if 1, w59:7..0 reflects the number of logical sectors currently set to transfer on READ/WRITE MULTIPLE command
	w63      Multiword DMA transfer modes
	w64      PIO transfer modes
	w65      minimum Multiword DMA transfer cycle time per word
	w66      device recommended Multiword DMA cycle time
	w67      minimum PIO transfer cycle time without IORDY flow control
	w67      minimum PIO transfer cycle time with IORDY flow control
	w75      queue depth

	w82:4    PACKET feature set is supported
	w85:4    PACKET feature set is supported

	w82:7    release interrupt is supported
	w85:7    release interrupt is enabled

	w82:8    SERVICE interrupt is supported
	w85:8    SERVICE interrupt is enabled

	w83:2    CFA feature set is supported
	w86:2    CFA feature set is supported

	w83:5    power-up in standby is supported
	w86:5    power-up in standby is enabled

	w83:6    device requires the SET FEATURES subcommand to spin-up after power-up if the Power-Up In Standby feature set is enabled (see 7.47.8)
	w86:6    device requires the SET FEATURES subcommand to spin-up after power-up

	w83:7    defined in Address Offset Reserved Area Boot, INCITS TR27:2001
	w86:7    defined in Address Offset Reserved Area Boot, INCITS TR27:2001

	w83:8    SET MAX security extension is supported
	w86:8    SET MAX security extension enabled

	w83:10   48-bit Address feature set is supported
	w86:10   48-bit Address feature set is supported

	w83:11   Device Configuration Overlay feature set is supported
	w86:11   Device Configuration Overlay feature set is supported

	w84:2    media serial number field is supported (words 176-205)
	w84:3    Media Card Pass Through Command feature set is supported
	w84:4    Streaming feature set is supported

	w84:13   IDLE IMMEDIATE with UNLOAD FEATURE is supported
	w87:13   IDLE IMMEDIATE with UNLOAD FEATURE is supported

	w119:1   Write-Read-Verify feature set is supported
	w120:1   Write-Read-Verify feature set is enabled

	w119:4   segmented feature for DOWNLOAD MICROCODE is supported
	w120:4   segmented feature for DOWNLOAD MICROCODE is supported

	w85:3    mandatory Power Management feature set is supported

	w87:2    media is present and media serial number in words 176-205 is valid
	w87:3    the Media Card Pass Through feature set is enabled
	w87:5    the device supports the General Purpose Logging feature set

	w88       Ultra DMA transfer modes, supported and currently selected
	w89       time required for Security erase unit completion
	w90       time required for Enhanced security erase unit completion
	w91       Advanced power management level value
	w92       Master Password Identifier
	w93       Hardware configuration test results
	w94       Current automatic acoustic management value
	w95       Stream Minimum Request Size
	w96       Streaming Transfer Time - DMA
	w97       Streaming Access Latency - DMA and PIO
	w98-99    Streaming Performance Granularity
	w104      streaming Transfer Time - PIO
	w107      Inter-seek delay for ISO 7779 standard acoustic testing
	w108-111  World wide name
	w128      Security status
	w160      CFA power mode
	w176-205  Current media serial number
	w206      SCT Command Transport
	w209      Alignment of logical blocks within a physical block
	w210-211  Write-Read-Verify Sector Count Mode 3 Only
	w212-213  Verify Sector Count Mode 2 Only
	w214      NV Cache Capabilities
	w215-216  NV Cache Size in Logical Blocks (MSW)
	w219      NV Cache Options
	w220      Write-Read-Verify Mode
	w222      Transport major revision number
	w223      Transport minor revision number
	w255      Integrity word (optional)

	XXX what about constant fields? E.g.:
	> Bit 15 of word 50 shall be cleared to zero to indicate that the contents of word 50 are valid.
	> Bit 14 of word 50 shall be set to one to indicate that the contents of word 50 are valid.
	> …
	> For PATA devices when bit 1 of word 53 is set to one, the values reported in words 64-70 are valid.
	> …
	> If bit 14 of word 83 is set to one and bit 15 of word 83 is cleared to zero, the contents of words 82-83 contain valid support information.
	> If not, support information is not valid in these words.
	> If bit 14 of word 84 is set to one and bit 15 of word 84 is cleared to zero, the contents of word 84 contains valid support information.
	> If not, support information is not valid in this word.
	> If bit 14 of word 119 is set to one and bit 15 of word 119 is cleared to zero, the contents of word 119 contains valid support information.
	> If not, support information is not valid in this word.
	> …
	> If bit 14 of word 87 is set to one and bit 15 of word 87 is cleared to zero, the contents of words 85-87 contain valid information.
	> If bit 14 of word 120 is set to one and bit 15 of word 120 is cleared to zero, the contents of word 120 contain valid information.
	> If not, information is not valid in these words.
	*/

	let sectors = ((data[61] as u64) << 16)
	            +  (data[60] as u64);
	let sectors_48bit = ((data[103] as u64) << 48)
	                  + ((data[102] as u64) << 32)
	                  + ((data[101] as u64) << 16)
	                  +  (data[100] as u64);

	// data[106] is valid if bit 14 is 1 and bit 15 is 0
	let sector_size_valid = data[106] & ((1<<14) + (1<<15)) == (1<<14);

	let sector_size_log = if sector_size_valid {
		(
			if data[106] & (1<<12) != 0 {
				// if bit 12 is 1, logical sector size is >256 words and determined by words 117-118
				((data[118] as u32) << 16) + (data[117] as u32)
			} else { 256 }
		) << 1 // convert words into bytes
	} else {
		// remember the times sectors were assumed to always be that long? yeah, those were the times…
		512
	};

	Id {
		is_ata: !is_set(data[0], 15),
		incomplete: is_set(data[0], 2),

		serial: read_string(&data, 10, 19),
		firmware: read_string(&data, 23, 26),
		model: read_string(&data, 27, 46),

		capacity: (sector_size_log as u64) * if sectors_48bit > 0 { sectors_48bit } else { sectors },

		sector_size_phy: if sector_size_valid {
			// bit 13 set to 1 indicates there's more than 1 logical sector per physical
			if data[106] & (1<<13) != 0 {
				// bits 0..3 of word 106 represent the size of physical sector in power of 2 logical sectors
				// (that is, 0x2 is 1<<0x2 logical sectors per physical)
				sector_size_log << (data[106] as u32 & 0xf)
			} else { sector_size_log }
		} else { 512 },
		sector_size_log: sector_size_log,

		rpm: match data[217] {
			// all values except 0x0000 are reserved (TODO warning?)
			0x0000 | 0xffff | 0x0002...0x0400 => RPM::Unknown,
			0x0001 => RPM::NonRotating,
			i => RPM::RPM(i),
		},

		trusted_computing_supported: is_set(data[48], 0),

		// TODO word 80: major revision number compatibility bits (if not 0x0000 nor 0xffff)
		ata_version: match data[81] {
			0x0001 ... 0x000c => Some("(obsolete)"),

			0x000d => Some("ATA/ATAPI-4 X3T13 1153D revision 6"),
			0x000e => Some("ATA/ATAPI-4 T13 1153D revision 13"),
			0x000f => Some("ATA/ATAPI-4 X3T13 1153D revision 7"),
			0x0010 => Some("ATA/ATAPI-4 T13 1153D revision 18"),
			0x0011 => Some("ATA/ATAPI-4 T13 1153D revision 15"),
			0x0012 => Some("ATA/ATAPI-4 published, ANSI INCITS 317-1998"),
			0x0013 => Some("ATA/ATAPI-5 T13 1321D revision 3"),
			0x0014 => Some("ATA/ATAPI-4 T13 1153D revision 14"),
			0x0015 => Some("ATA/ATAPI-5 T13 1321D revision 1"),
			0x0016 => Some("ATA/ATAPI-5 published, ANSI INCITS 340-2000"),
			0x0017 => Some("ATA/ATAPI-4 T13 1153D revision 17"),
			0x0018 => Some("ATA/ATAPI-6 T13 1410D revision 0"),
			0x0019 => Some("ATA/ATAPI-6 T13 1410D revision 3a"),
			0x001a => Some("ATA/ATAPI-7 T13 1532D revision 1"),
			0x001b => Some("ATA/ATAPI-6 T13 1410D revision 2"),
			0x001c => Some("ATA/ATAPI-6 T13 1410D revision 1"),
			0x001d => Some("ATA/ATAPI-7 published ANSI INCITS 397-2005"),
			0x001e => Some("ATA/ATAPI-7 T13 1532D revision 0"),
			0x001f => Some("ACS-3 Revision 3b"),

			0x0021 => Some("ATA/ATAPI-7 T13 1532D revision 4a"),
			0x0022 => Some("ATA/ATAPI-6 published, ANSI INCITS 361-2002"),

			0x0027 => Some("ATA8-ACS revision 3c"),
			0x0028 => Some("ATA8-ACS revision 6"),
			0x0029 => Some("ATA8-ACS revision 4"),

			0x0031 => Some("ACS-2 revision 2"),

			0x0033 => Some("ATA8-ACS revision 3e"),

			0x0039 => Some("ATA8-ACS revision 4c"),

			0x0042 => Some("ATA8-ACS revision 3f"),

			0x0052 => Some("ATA8-ACS revision 3b"),

			0x005e => Some("ACS-4 revision 5"),

			0x006d => Some("ACS-3 revision 5"),

			0x0082 => Some("ACS-2 published, ANSI INCITS 482-2012"),

			0x0107 => Some("ATA8-ACS revision 2d"),

			0x010a => Some("ACS-3 published, ANSI INCITS 522-2014"),

			0x0110 => Some("ACS-2 revision 3"),

			0x011b => Some("ACS-3 revision 4"),
			0x0000 | 0xffff => None, // revision is not reported
			_ => None, // reserved values
		},

		commands_supported: IdCommands {
			// XXX these, according to ATA8-ACS rev 62, should be mirrored in 'feature status' words
			// e.g. w82:12 == w85:12, w119:2 == w120:2
			// 82 ↔ 85, 83 ↔ 86, 84 ↔ 87, 119 ↔ 120
			device_reset: is_set(data[82], 9),
			write_buffer: is_set(data[82], 12),
			read_buffer: is_set(data[82], 13),
			nop: is_set(data[82], 14),
			download_microcode: is_set(data[83], 0),
			read_write_dma_queued: is_set(data[83], 1),
			flush_cache: is_set(data[83], 12),
			flush_cache_ext: is_set(data[83], 13),
			write_dma_fua_ext: is_set(data[84], 6),
			write_dma_queued_fua_ext: is_set(data[84], 7),
			write_uncorrectable: is_set(data[119], 2),
			read_write_dma_ext_gpl: is_set(data[119], 3),
		},

		power_mgmt_supported: is_set(data[82], 3),
		write_cache: make_ternary(&data, 82, 5, 85, 5),
		read_look_ahead: make_ternary(&data, 82, 6, 85, 6),
		/* TODO
		> the device is not indicating its full size as defined by READ NATIVE MAX or READ NATIVE MAX EXT command
		> because a SET MAX ADDESS or SET MAX ADDRESS EXT command has been issued to resize the device
		which is indicated by hpa == Enabled
		*/
		hpa: make_ternary(&data, 82, 10, 85, 10),
		apm: make_ternary(&data, 83, 3, 86, 3),
		aam: make_ternary(&data, 83, 9, 86, 9),
		gp_logging_supported: is_set(data[84], 5),
		wwn_supported: is_set(data[84], 8), // XXX mirrored; see commands_supported
		security: make_ternary(&data, 82, 1, 85, 1),

		smart: make_ternary(&data, 82, 0, 85, 0),

		smart_error_logging_supported: is_set(data[84], 0), // XXX mirrored; see commands_supported
		smart_self_test_supported: is_set(data[84], 1), // XXX mirrored; see commands_supported
	}
}
