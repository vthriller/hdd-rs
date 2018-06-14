// XXX seems like Cargo has no support for target-specific build scripts…

extern crate bindgen;

use std::env;
use std::path::PathBuf;

#[cfg(target_os = "freebsd")]
fn main() {
	println!("cargo:rustc-link-lib=cam");

	let bindings = bindgen::Builder::default()
		.header("bindgen-freebsd.h")
		.whitelisted_function("cam_(open|close)_device")
		.whitelisted_function("cam_(get|free)ccb")
		.whitelisted_function("cam_send_ccb")
		.whitelisted_function("cam_error_string")
		.whitelisted_type("ccb_flags")
		.whitelisted_type("cam_status")
		.whitelisted_type("cam_error_string_flags")
		.whitelisted_type("cam_error_proto_flags")
		.whitelisted_type("ccb_dev_match_status")
		.whitelisted_type("dev_match_result")
		.whitelisted_var("cam_errbuf")
		.whitelisted_var("CAM_ATAIO_.*")
		.whitelisted_var("MSG_SIMPLE_Q_TAG")
		.whitelisted_var("XPT_DEVICE")
		// XXX see bindings.rs for the hack
		//.whitelisted_var("CAM_XPT_PATH_ID")
		//.whitelisted_var("CAM_(TARGET|LUN)_WILDCARD")
		//.whitelisted_var("CAMIOCOMMAND")
		.generate()
		.expect("Unable to generate bindings");

	let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings
		.write_to_file(out_path.join("bindings.rs"))
		.expect("Couldn't write bindings!");
}

#[cfg(target_os = "linux")]
fn main() {
	let bindings = bindgen::Builder::default()
		.header("bindgen-linux.h")
		.whitelisted_var("bindgen_SG_IO")
		.generate()
		.expect("Unable to generate bindings");

	let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings
		.write_to_file(out_path.join("bindings.rs"))
		.expect("Couldn't write bindings!");
}
