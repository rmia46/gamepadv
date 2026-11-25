pub mod capture;
pub mod discovery;

pub use capture::InputRig;
pub use discovery::{
    build_passthrough_keyboard, classify_capabilities, classify_device, probe_input_devices,
    standard_keyboard_keys, DeviceClass, DiscoveryReport, GrabGuard, PASSTHROUGH_KEYBOARD_NAME,
};
