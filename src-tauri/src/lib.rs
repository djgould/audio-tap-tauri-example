use std::ffi::{c_char, CStr};
use std::mem;
use std::ops::DerefMut;
use std::ptr::{self, null};

use core_foundation::array::CFArray;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use coreaudio_sys::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceMainSubDeviceKey,
    kAudioAggregateDeviceNameKey, kAudioAggregateDeviceSubDeviceListKey,
    kAudioAggregateDeviceTapAutoStartKey, kAudioAggregateDeviceTapListKey,
    kAudioAggregateDeviceUIDKey, kAudioDevicePropertyDeviceUID, kAudioDevicePropertyScopeOutput,
    kAudioHardwareNoError, kAudioHardwarePropertyDefaultSystemOutputDevice,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyElementMaster,
    kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject, kAudioSubDeviceUIDKey,
    kAudioSubTapDriftCompensationKey, kAudioSubTapUIDKey, kCFStringEncodingUTF8, AudioBufferList,
    AudioDeviceCreateIOProcID, AudioDeviceID, AudioDeviceIOProcID, AudioDeviceStart,
    AudioHardwareCreateAggregateDevice, AudioObjectGetPropertyData, AudioObjectID,
    AudioObjectPropertyAddress, AudioTimeStamp, CFDictionaryRef, CFStringGetCStringPtr,
    CFStringRef, OSStatus,
};

use objc::runtime::{Class, Object};
use objc::{msg_send, sel, sel_impl};
use objc_foundation::{INSArray, INSObject, INSString, NSArray, NSString};
use objc_id::{Id, Owned};
use uuid::Uuid;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[repr(i64)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CATapMuteBehavior {
    Unmuted = 0,
    Muted = 1,
    MutedWhenTapped = 2,
}

pub struct CATapDescription {
    pub obj: Id<Object, Owned>,
}

impl CATapDescription {
    pub fn new_mono_global_tap_but_exclude(processes: Vec<i32>) -> Self {
        unsafe {
            let class = Class::get("CATapDescription").unwrap();
            let obj: *mut Object = msg_send![class, alloc];
            let nsarray =
                NSArray::from_vec(processes.iter().map(|&id| NSNumber::new(id)).collect());
            let obj: *mut Object = msg_send![obj, initMonoGlobalTapButExcludeProcesses: nsarray];
            Self {
                obj: Id::from_ptr(obj),
            }
        }
    }

    pub fn get_uuid(&self) -> Uuid {
        unsafe {
            let nsuuid: *mut Object = msg_send![self.obj, UUID];
            let uuid_string: Id<NSString> = msg_send![nsuuid, UUIDString];
            let rust_string: String = uuid_string.as_str().to_owned();
            Uuid::parse_str(&rust_string).expect("Failed to parse UUID")
        }
    }
}

// Wrapper for NSNumber
struct NSNumber {}

impl NSNumber {
    pub fn new(value: i32) -> Id<NSNumber> {
        unsafe {
            let class = Class::get("NSNumber").unwrap();
            let obj: *mut Object = msg_send![class, numberWithInt: value];
            Id::from_ptr(obj as *mut NSNumber)
        }
    }
}

unsafe impl objc::Message for NSNumber {}

impl INSObject for NSNumber {
    fn class() -> &'static Class {
        Class::get("NSNumber").unwrap()
    }
}

extern "C" {
    fn AudioHardwareCreateProcessTap(
        inDescription: *mut Object,
        outTapID: *mut AudioObjectID,
    ) -> OSStatus;
}

fn cfstring_from_bytes_with_nul(bytes: &'static [u8]) -> CFString {
    let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(bytes) };
    CFString::new(cstr.to_str().unwrap())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut tap_id: AudioObjectID = 0;
    let mut aggregate_id = 0;
    let mut tap_description = CATapDescription::new_mono_global_tap_but_exclude(vec![]);
    let status: OSStatus;
    unsafe {
        status = AudioHardwareCreateProcessTap(
            tap_description.obj.deref_mut() as *const _ as *mut _,
            &mut tap_id,
        );
    }

    if status != 0 {
        println!("error creating proces tap");
        return;
    }

    println!("created process tap! id {}", tap_id);

    let address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDefaultSystemOutputDevice,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };

    let system_output_id: AudioDeviceID = 0;
    let data_size = mem::size_of::<AudioDeviceID>();

    let status = unsafe {
        AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &address as *const _,
            0,
            null(),
            &data_size as *const _ as *mut _,
            &system_output_id as *const _ as *mut _,
        )
    };
    if status != kAudioHardwareNoError as i32 {
        println!("Error getting default device");
        return;
    }

    println!("got default audio device id");

    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyDeviceUID,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMaster,
    };

    let output_uid: CFStringRef = null();
    let data_size = mem::size_of::<CFStringRef>();
    let status = unsafe {
        AudioObjectGetPropertyData(
            system_output_id,
            &address as *const _,
            0,
            null(),
            &data_size as *const _ as *mut _,
            &output_uid as *const _ as *mut _,
        )
    };

    if status != 0 {
        println!("failed to get device uid");
        return;
    }
    let c_string: *const c_char =
        unsafe { CFStringGetCStringPtr(output_uid, kCFStringEncodingUTF8) };
    let output_uid_str = unsafe { CStr::from_ptr(c_string).to_string_lossy().into_owned() };
    println!("Got device uid {}", output_uid_str);

    let aggregate_device_name = CFString::new("Tap-1234");
    let aggregate_device_uid = CFString::new("tap-1234567-uid");
    let output_uid_cfstr = CFString::new(&output_uid_str);

    // Sub-device UID key and dictionary
    let sub_device_dict = CFDictionary::from_CFType_pairs(&[(
        cfstring_from_bytes_with_nul(kAudioSubDeviceUIDKey).as_CFType(),
        output_uid_cfstr.as_CFType(),
    )]);

    let tap_uuid_string = CFString::new(&tap_description.get_uuid().to_string());

    println!("tap_uuid_string {}", tap_uuid_string.to_string());

    let tap_device_dict = CFDictionary::from_CFType_pairs(&[
        (
            cfstring_from_bytes_with_nul(kAudioSubTapDriftCompensationKey).as_CFType(),
            CFBoolean::false_value().as_CFType(),
        ),
        (
            cfstring_from_bytes_with_nul(kAudioSubTapUIDKey).as_CFType(),
            tap_uuid_string.as_CFType(),
        ),
    ]);

    // Sub-device list
    let sub_device_list = CFArray::from_CFTypes(&[sub_device_dict]);

    let tap_list = CFArray::from_CFTypes(&[tap_device_dict]);

    // Create the aggregate device description dictionary
    let description_dict = CFDictionary::from_CFType_pairs(&[
        (
            cfstring_from_bytes_with_nul(kAudioAggregateDeviceNameKey).as_CFType(),
            aggregate_device_name.as_CFType(),
        ),
        (
            cfstring_from_bytes_with_nul(kAudioAggregateDeviceUIDKey).as_CFType(),
            aggregate_device_uid.as_CFType(),
        ),
        (
            cfstring_from_bytes_with_nul(kAudioAggregateDeviceMainSubDeviceKey).as_CFType(),
            output_uid_cfstr.as_CFType(),
        ),
        (
            cfstring_from_bytes_with_nul(kAudioAggregateDeviceIsPrivateKey).as_CFType(),
            CFBoolean::true_value().as_CFType(),
        ),
        (
            cfstring_from_bytes_with_nul(kAudioAggregateDeviceTapAutoStartKey).as_CFType(),
            CFBoolean::true_value().as_CFType(),
        ),
        (
            cfstring_from_bytes_with_nul(kAudioAggregateDeviceSubDeviceListKey).as_CFType(),
            sub_device_list.as_CFType(),
        ),
        (
            cfstring_from_bytes_with_nul(kAudioAggregateDeviceTapListKey).as_CFType(),
            tap_list.as_CFType(),
        ),
    ]);

    // Convert the dictionary to CFDictionaryRef
    let aggregate_device_uid = description_dict.as_concrete_TypeRef() as CFDictionaryRef;

    // Initialize the aggregate device ID
    let mut aggregate_device_id: AudioObjectID = 0;

    // Call AudioHardwareCreateAggregateDevice
    let status = unsafe {
        AudioHardwareCreateAggregateDevice(aggregate_device_uid, &mut aggregate_device_id)
    };

    // Check the status and return the appropriate result
    if status != 0 {
        println!("failed to create aggregate device");
        return;
    }

    println!("created aggregate device {}", aggregate_device_id);

    unsafe extern "C" fn io_proc(
        _in_device: AudioObjectID,
        _in_now: *const AudioTimeStamp,
        in_input_data: *const AudioBufferList,
        _in_input_time: *const AudioTimeStamp,
        _out_output_data: *mut AudioBufferList,
        _in_output_time: *const AudioTimeStamp,
        _in_client_data: *mut ::std::os::raw::c_void,
    ) -> OSStatus {
        println!(
            "input proc {}",
            (*in_input_data).mNumberBuffers, // this value should be 1 if it is working
        );
        return 0;
    }

    let mut device_proc_id: AudioDeviceIOProcID = None;

    println!("Run tap!");

    let err = unsafe {
        AudioDeviceCreateIOProcID(
            aggregate_device_id,
            Some(io_proc),
            ptr::null_mut(),
            &mut device_proc_id,
        )
    };

    if err != 0 {
        println!("failed to create io proc");
        return;
    }

    println!("created io proc");

    let err = unsafe { AudioDeviceStart(aggregate_device_id, device_proc_id) };

    if err != 0 {
        println!("failed to start device");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
