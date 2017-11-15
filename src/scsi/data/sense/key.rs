/// Sense key descriptions, as seen in SPC-4, 4.5.6, table 43
#[derive(Debug)]
pub enum SenseKey {
	/// No Sense: indicates successful command execution, or might occur for a command that received CHECK CONDITION status because one of FILEMARK/EOM/ILI bits was set
	Ok = 0,
	/// Recovered Error: still indicates successfully executed command but with some recovery action performed
	Recovered = 1,
	/// Not Ready: the logical unit is not accessible
	NotReady = 2,
	/// Medium Error: usually indicates unrecoverable errors caused by damaged media
	MediumError = 3,
	/// Hardware Error: unrecoverable non-medium failure (controller/CRC/…)
	HardwareError = 4,
	/// Illegal Request: invalid LUN/task attribute/parameter/…
	///
	/// If invalid parameter is not in the CDB, device might return this *after* altering medium in some way.
	IllegalRequest = 5,
	/// Unit Attention: removable medium change, logical unit reset etc.; see SAM-4
	UnitAttention = 6,
	/// Data Protect: indicates prohibited read/write operations on protected blocks
	DataProtect = 7,
	/// Blank Check: blank medium or format-defined end-of-data was encountered during reading, or non-blank medium was encountered during writing
	BlankCheck = 8,
	/// aka Firmware Error
	VendorSpecific = 9,
	/// Copy Aborted: indicates aborted EXTENDED COPY command
	AbortedCopy = 10,
	/// Aborted Command: indicates any other aborted command; client may be able to recover by trying the command again
	AbortedCommand = 11,
	Reserved = 12,
	/// Volume Overflow: indicates that buffered device reached the end-of-partition; RECOVER BUFFERED DATA might be used to read unwritten data back (see SSC-2)
	VolumeOverflow = 13,
	/// Miscompare: source data did not match the data read from the medium
	Miscompare = 14,
	/// Completed: completion sense data report; also may occur for successful command
	Completed = 15,
}
