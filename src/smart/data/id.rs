fn bytes_to_words(data: &[u8; 512]) -> [u16; 256] {
	// XXX mut?
	let mut output: [u16; 256] = [0; 256];

	for i in 0..256 {
		if cfg!(target_endian = "little") {
			output[i]  = (data[2 * i + 1] as u16) << 8;
			output[i] += data[2 * i] as u16;
		} else {
			output[i]  = (data[2 * i] as u16) << 8;
			output[i] += data[2 * i + 1] as u16;
		}
	}

	output
}

// TODO make sure characters are in the range of 0x20 to (and including) 0x7e
// (this is in the standard, and also to make std::String safe again)
fn read_string(arr: [u16; 256], start: usize, fin: usize) -> String {
	let mut output = String::with_capacity((fin - start) * 2);

	for i in start..fin {
		output.push((arr[i] >> 8) as u8 as char);
		output.push((arr[i] & 0xff) as u8 as char);
	}

	String::from(output.trim())
}

#[derive(Debug)]
pub enum Ternary {
	Unsupported, Disabled, Enabled
}

#[derive(Debug)]
pub struct IdCommands {
	device_reset: bool,
	write_buffer: bool,
	read_buffer: bool,
	nop:  bool,
	download_microcode: bool,
	read_write_dma_queued: bool,
	flush_cache: bool,
	flush_cache_ext: bool,
	write_dma_fua_ext: bool, // also WRITE MULTIPLE FUA EXT
	write_dma_queued_fua_ext: bool,
	write_uncorrectable: bool,
	read_write_dma_ext_gpl: bool,
}

#[derive(Debug)]
pub struct Id {
	is_ata: bool, // probably redundant
	incomplete: bool, // content of words other that 0 or 2 might be invalid

	serial: String,
	firmware: String,
	model: String,

	trusted_computing_supported: bool,

	ata_version: Option<&'static str>,

	commands_supported: IdCommands,

	power_mgmt_supported: bool,
	write_cache: Ternary,
	read_look_ahead: Ternary,
	hpa: Ternary, // Host Protected Area
	apm: Ternary, // Advanced Power Management
	aam: Ternary, // Automatic Acoustic Management
	gp_logging_supported: bool, // General Purpose Logging
	wwn_supported: bool, // World Wide Name
	security: Ternary,

	smart: Ternary,
	smart_error_logging_supported: bool,
	smart_self_test_supported: bool,
}

fn is_set(word: u16, bit: usize) -> bool {
	word & (1<<bit) != 0
}
fn make_ternary(data: [u16; 256], word_sup: usize, bit_sup: usize, word_enabled: usize, bit_enabled: usize) -> Ternary {
	if !is_set(data[word_sup], bit_sup) {
		Ternary::Unsupported
	} else {
		if is_set(data[word_enabled], bit_enabled) { Ternary::Enabled }
		else { Ternary::Disabled }
	}
}

pub fn parse_id(data: &[u8; 512]) -> Id {
	let data = bytes_to_words(data);
	/*
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
	w60..61  number of user addressable sectors
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
	w100-103  Total Number of User Addressable Sectors for the 48-bit Address feature set
	w104      streaming Transfer Time - PIO
	w106      physical sector size / Logical Sector Size
	w107      Inter-seek delay for ISO 7779 standard acoustic testing
	w108-111  World wide name
	w117-118  Logical Sector Size
	w128      Security status
	w160      CFA power mode
	w176-205  Current media serial number
	w206      SCT Command Transport
	w209      Alignment of logical blocks within a physical block
	w210-211  Write-Read-Verify Sector Count Mode 3 Only
	w212-213  Verify Sector Count Mode 2 Only
	w214      NV Cache Capabilities
	w215-216  NV Cache Size in Logical Blocks (MSW)
	w217      NV Cache Read Transfer Speed in MB/s
	w218      NV Cache Write Transfer Speed in MB/s
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
	Id {
		is_ata: !is_set(data[0], 15),
		incomplete: is_set(data[0], 2),

		serial: read_string(data, 10, 19),
		firmware: read_string(data, 23, 26),
		model: read_string(data, 27, 46),

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
			0x0021 => Some("ATA/ATAPI-7 T13 1532D revision 4a"),
			0x0022 => Some("ATA/ATAPI-6 published, ANSI INCITS 361-2002"),
			0x0027 => Some("ATA8-ACS revision 3c"),
			0x0033 => Some("ATA8-ACS revision 3e"),
			0x0042 => Some("ATA8-ACS revision 3f"),
			0x0052 => Some("ATA8-ACS revision 3b"),
			0x0107 => Some("ATA8-ACS revision 2d"),
			0x0000 | 0xffff => None, // revision is not reported
			_ => None, // reserved values
		},

		commands_supported: IdCommands {
			/* XXX
			> If bit 9 of word 85 is set to one, the DEVICE RESET command is supported.
			Should we set w82:9 || w85:9 here? Or w82:9 && w85:9? Or is it `Ternary`?
			Lousy stupid drafts…
			*/
			device_reset: is_set(data[82], 9),
			// XXX same, w82:12 vs w85:12
			write_buffer: is_set(data[82], 12),
			// XXX same, w82:13 vs w85:13
			read_buffer: is_set(data[82], 13),
			// XXX same, w82:14 vs w85:14
			nop: is_set(data[82], 14),
			// XXX same, w83:0 vs w86:0
			download_microcode: is_set(data[83], 0),
			// XXX same, w83:1 vs w86:1
			read_write_dma_queued: is_set(data[83], 1),
			// XXX same, w83:12 vs w86:12
			flush_cache: is_set(data[83], 12),
			// XXX same, w83:13 vs w86:13
			flush_cache_ext: is_set(data[83], 13),
			// XXX same, w84:6 vs w87:6
			write_dma_fua_ext: is_set(data[84], 6),
			// XXX same, w84:7 vs w87:7
			write_dma_queued_fua_ext: is_set(data[84], 7),
			// XXX same, w119:2 vs w120:2
			write_uncorrectable: is_set(data[119], 2),
			// XXX same, w119:3 vs w120:3
			read_write_dma_ext_gpl: is_set(data[119], 3),
		},

		power_mgmt_supported: is_set(data[82], 3),
		write_cache: make_ternary(data, 82, 5, 85, 5),
		read_look_ahead: make_ternary(data, 82, 6, 85, 6),
		/* TODO
		> the device is not indicating its full size as defined by READ NATIVE MAX or READ NATIVE MAX EXT command
		> because a SET MAX ADDESS or SET MAX ADDRESS EXT command has been issued to resize the device
		which is indicated by hpa == Enabled
		*/
		hpa: make_ternary(data, 82, 10, 85, 10),
		apm: make_ternary(data, 83, 3, 86, 3),
		aam: make_ternary(data, 83, 9, 86, 9),
		gp_logging_supported: is_set(data[84], 5),
		// XXX see commands_supported; w84:8 vs w87:8
		wwn_supported: is_set(data[84], 8),
		security: make_ternary(data, 82, 1, 85, 1),

		smart: make_ternary(data, 82, 0, 85, 0),
		/* XXX
		> If bit 0 of word 87 is set to one, the device supports SMART error logging.
		> If bit 1 of word 87 is set to one, the device supports SMART self-test.

		Oh, come on!
		*/
		smart_error_logging_supported: is_set(data[84], 0),
		smart_self_test_supported: is_set(data[84], 1),
	}
}
