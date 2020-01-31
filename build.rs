// XXX seems like Cargo has no support for target-specific build scripts…

#[cfg(target_os = "freebsd")]
extern crate bindgen;

#[cfg(target_os = "freebsd")]
use std::env;
#[cfg(target_os = "freebsd")]
use std::path::PathBuf;

#[cfg(target_os = "freebsd")]
fn main() {
	println!("cargo:rustc-link-lib=cam");

	let bindings = bindgen::Builder::default()
		.header("bindgen-freebsd.h")
		.whitelist_function("cam_(open|close)_device")
		.whitelist_function("cam_(get|free)ccb")
		.whitelist_function("cam_send_ccb")
		.whitelist_function("cam_error_string")
		.whitelist_type("ccb_flags")
		.whitelist_type("cam_status")
		.whitelist_type("cam_error_string_flags")
		.whitelist_type("cam_error_proto_flags")
		.whitelist_type("ccb_dev_match_status")
		.whitelist_type("dev_match_result")
		.whitelist_var("cam_errbuf")
		.whitelist_var("CAM_ATAIO_.*")
		.whitelist_var("MSG_SIMPLE_Q_TAG")
		.whitelist_var("XPT_DEVICE")
		// XXX see bindings.rs for the hack
		//.whitelist_var("CAM_XPT_PATH_ID")
		//.whitelist_var("CAM_(TARGET|LUN)_WILDCARD")
		//.whitelist_var("CAMIOCOMMAND")
		.generate()
		.expect("Unable to generate bindings");

	let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings
		.write_to_file(out_path.join("bindings.rs"))
		.expect("Couldn't write bindings!");
}

#[cfg(not(target_os = "freebsd"))]
fn main() {
}
