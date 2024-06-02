use std::ops::Deref;

use coreaudio_sys::{AudioObjectID, OSStatus};
use objc::runtime::Object;

use super::ca_tap_description::CATapDescription;

// Import the Objective-C function
extern "C" {
    fn AudioHardwareCreateProcessTap(
        inDescription: *mut Object,
        outTapID: *mut AudioObjectID,
    ) -> OSStatus;
}

// Rust function to create a process tap
pub fn audio_hardware_create_process_tap(description: &CATapDescription) -> Result<u32, i32> {
    let mut tap_id: AudioObjectID = 0;
    let status: OSStatus;
    unsafe {
        status = AudioHardwareCreateProcessTap(
            description.obj.deref() as *const _ as *mut _,
            &mut tap_id,
        );
    }
    if status == 0 {
        Ok(tap_id)
    } else {
        Err(status)
    }
}
