//! Test that demonstrates the CLAP plugin receiving track name from host.
//!
//! This test loads the skipper.clap plugin as a dynamic library and acts as
//! a minimal CLAP host that provides the track-info extension with a test
//! track name.

use clap_sys::entry::clap_plugin_entry;
use clap_sys::ext::track_info::{
    clap_host_track_info, clap_track_info, CLAP_EXT_TRACK_INFO, CLAP_TRACK_INFO_HAS_TRACK_COLOR,
    CLAP_TRACK_INFO_HAS_TRACK_NAME,
};
use clap_sys::factory::plugin_factory::{clap_plugin_factory, CLAP_PLUGIN_FACTORY_ID};
use clap_sys::host::clap_host;
use clap_sys::plugin::clap_plugin;
use clap_sys::version::CLAP_VERSION;
use libloading::Library;
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

/// Test track name that the mock host provides
const TEST_TRACK_NAME: &str = "My Awesome Track";
const TEST_TRACK_COLOR: (u8, u8, u8) = (255, 128, 64); // Orange

/// Flag to track if track_info::get was called
static TRACK_INFO_GET_CALLED: AtomicBool = AtomicBool::new(false);

/// The track-info extension vtable
static HOST_TRACK_INFO: clap_host_track_info = clap_host_track_info {
    get: Some(host_track_info_get),
};

/// Implementation of clap_host_track_info::get
unsafe extern "C" fn host_track_info_get(
    _host: *const clap_host,
    info: *mut clap_track_info,
) -> bool {
    TRACK_INFO_GET_CALLED.store(true, Ordering::SeqCst);

    if info.is_null() {
        return false;
    }

    let info = &mut *info;

    // Set flags
    info.flags = CLAP_TRACK_INFO_HAS_TRACK_NAME | CLAP_TRACK_INFO_HAS_TRACK_COLOR;

    // Set track name
    let name_bytes = TEST_TRACK_NAME.as_bytes();
    let copy_len = name_bytes.len().min(info.name.len() - 1);
    for (i, &byte) in name_bytes[..copy_len].iter().enumerate() {
        info.name[i] = byte as c_char;
    }
    info.name[copy_len] = 0; // Null terminate

    // Set color
    info.color.red = TEST_TRACK_COLOR.0;
    info.color.green = TEST_TRACK_COLOR.1;
    info.color.blue = TEST_TRACK_COLOR.2;
    info.color.alpha = 255;

    println!(
        "host_track_info_get called! Returning track name: '{}', color: {:?}",
        TEST_TRACK_NAME, TEST_TRACK_COLOR
    );

    true
}

/// Implementation of clap_host::get_extension
unsafe extern "C" fn host_get_extension(
    _host: *const clap_host,
    extension_id: *const c_char,
) -> *const c_void {
    if extension_id.is_null() {
        return ptr::null();
    }

    let ext_id = CStr::from_ptr(extension_id);

    if ext_id == CLAP_EXT_TRACK_INFO {
        println!("Plugin requested track-info extension - providing it!");
        &HOST_TRACK_INFO as *const _ as *const c_void
    } else {
        println!("Plugin requested extension: {:?} - not provided", ext_id);
        ptr::null()
    }
}

unsafe extern "C" fn host_request_restart(_host: *const clap_host) {
    println!("Plugin requested restart");
}

unsafe extern "C" fn host_request_process(_host: *const clap_host) {
    println!("Plugin requested process");
}

unsafe extern "C" fn host_request_callback(_host: *const clap_host) {
    println!("Plugin requested callback");
}

#[test]
fn test_plugin_receives_track_name() {
    // Find the built plugin
    let plugin_path = find_plugin_path();
    println!("Loading plugin from: {:?}", plugin_path);

    // Load the plugin library
    let lib = unsafe { Library::new(&plugin_path) }.expect("Failed to load plugin library");

    // Get the CLAP entry point
    let entry: libloading::Symbol<*const clap_plugin_entry> =
        unsafe { lib.get(b"clap_entry") }.expect("Failed to find clap_entry symbol");

    let entry = unsafe { &**entry };
    println!(
        "CLAP version: {}.{}.{}",
        entry.clap_version.major, entry.clap_version.minor, entry.clap_version.revision
    );

    // Initialize the plugin entry
    let plugin_path_cstr = CString::new(plugin_path.to_str().unwrap()).unwrap();
    let init_result = unsafe { (entry.init.unwrap())(plugin_path_cstr.as_ptr()) };
    assert!(init_result, "Plugin init failed");

    // Get the plugin factory
    let factory_ptr = unsafe { (entry.get_factory.unwrap())(CLAP_PLUGIN_FACTORY_ID.as_ptr()) };
    assert!(!factory_ptr.is_null(), "Failed to get plugin factory");

    let factory = unsafe { &*(factory_ptr as *const clap_plugin_factory) };

    // Get plugin count
    let count = unsafe { (factory.get_plugin_count.unwrap())(factory) };
    println!("Plugin count: {}", count);
    assert!(count > 0, "No plugins found in factory");

    // Get plugin descriptor
    let descriptor = unsafe { (factory.get_plugin_descriptor.unwrap())(factory, 0) };
    assert!(!descriptor.is_null(), "Failed to get plugin descriptor");

    let descriptor = unsafe { &*descriptor };
    let plugin_id = unsafe { CStr::from_ptr(descriptor.id) };
    let plugin_name = unsafe { CStr::from_ptr(descriptor.name) };
    println!(
        "Found plugin: {} ({})",
        plugin_name.to_str().unwrap(),
        plugin_id.to_str().unwrap()
    );

    // Create our mock host
    let host_name = CString::new("Track Info Test Host").unwrap();
    let host_vendor = CString::new("Skipper Tests").unwrap();
    let host_url = CString::new("https://github.com/bedwards/skipper").unwrap();
    let host_version = CString::new("1.0.0").unwrap();

    let host = Box::new(clap_host {
        clap_version: CLAP_VERSION,
        host_data: ptr::null_mut(),
        name: host_name.as_ptr(),
        vendor: host_vendor.as_ptr(),
        url: host_url.as_ptr(),
        version: host_version.as_ptr(),
        get_extension: Some(host_get_extension),
        request_restart: Some(host_request_restart),
        request_process: Some(host_request_process),
        request_callback: Some(host_request_callback),
    });

    // Create plugin instance
    let plugin_ptr =
        unsafe { (factory.create_plugin.unwrap())(factory, &*host as *const _, descriptor.id) };
    assert!(!plugin_ptr.is_null(), "Failed to create plugin instance");

    let plugin = unsafe { &*plugin_ptr };
    println!("Plugin instance created successfully");

    // Init the plugin - this queries host extensions
    let init_success = unsafe { (plugin.init.unwrap())(plugin_ptr) };
    assert!(init_success, "Plugin init failed");
    println!("Plugin init complete (extensions queried)");

    // Activate the plugin - this calls plugin.initialize() which should query track_info
    // sample_rate=48000, min_frames=32, max_frames=1024
    let activate_success = unsafe { (plugin.activate.unwrap())(plugin_ptr, 48000.0, 32, 1024) };
    assert!(activate_success, "Plugin activation failed");
    println!("Plugin activated");

    // Verify that track_info::get was called
    assert!(
        TRACK_INFO_GET_CALLED.load(Ordering::SeqCst),
        "Plugin did not call host_track_info::get() during activation!"
    );

    println!("SUCCESS: Plugin called track_info::get and received track name '{}'", TEST_TRACK_NAME);

    // Clean up - deactivate before destroy
    unsafe { (plugin.deactivate.unwrap())(plugin_ptr) };
    unsafe { (plugin.destroy.unwrap())(plugin_ptr) };
    unsafe { (entry.deinit.unwrap())() };

    // Keep CStrings alive until after cleanup
    drop(host);
    drop(host_name);
    drop(host_vendor);
    drop(host_url);
    drop(host_version);
}

fn find_plugin_path() -> PathBuf {
    // Look for the built plugin in target/bundled
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bundled_path = PathBuf::from(manifest_dir)
        .join("target")
        .join("bundled")
        .join("skipper.clap");

    if bundled_path.exists() {
        // On macOS, .clap is a bundle directory
        #[cfg(target_os = "macos")]
        {
            bundled_path
                .join("Contents")
                .join("MacOS")
                .join("skipper")
        }
        #[cfg(not(target_os = "macos"))]
        {
            bundled_path
        }
    } else {
        panic!(
            "Plugin not found at {:?}. Run 'cargo xtask bundle skipper --release' first.",
            bundled_path
        );
    }
}
