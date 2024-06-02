extern crate objc;
extern crate objc_foundation;
extern crate objc_id;
extern crate uuid;

use objc::runtime::{Class, Object};
use objc::{msg_send, sel, sel_impl};
use objc_foundation::{INSArray, INSObject, INSString, NSArray, NSString};
use objc_id::{Id, Owned};
use uuid::Uuid;

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
    pub fn new_stereo_mixdown(processes: Vec<i32>) -> Self {
        unsafe {
            let class = Class::get("CATapDescription").unwrap();
            let obj: *mut Object = msg_send![class, alloc];
            let nsarray =
                NSArray::from_vec(processes.iter().map(|&id| NSNumber::new(id)).collect());
            let obj: *mut Object = msg_send![obj, initStereoMixdownOfProcesses: nsarray];
            Self {
                obj: Id::from_ptr(obj),
            }
        }
    }

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

    pub fn set_name(&self, name: &str) {
        let nsstring = NSString::from_str(name);
        unsafe {
            let _: () = msg_send![self.obj, setName: nsstring];
        }
    }

    pub fn set_uuid(&self, uuid: Uuid) {
        let nsuuid = NSUUID::from_uuid(uuid);
        unsafe {
            let _: () = msg_send![self.obj, setUUID: nsuuid];
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

    pub fn set_mute_behavior(&self, behavior: CATapMuteBehavior) {
        unsafe {
            let _: () = msg_send![self.obj, setMuteBehavior: behavior as i64];
        }
    }

    pub fn is_exclusive(&self) -> bool {
        unsafe {
            let is_exclusive: bool = msg_send![self.obj, isExclusive];
            is_exclusive
        }
    }
}

// Wrapper for NSNumber
pub struct NSNumber {
    obj: Id<Object, Owned>,
}

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

// Custom NSUUID wrapper
pub struct NSUUID {
    obj: Id<Object, Owned>,
}

impl NSUUID {
    pub fn from_uuid(uuid: Uuid) -> Id<NSUUID> {
        let uuid_string = uuid.to_string();
        let nsstring = NSString::from_str(&uuid_string);
        unsafe {
            let class = Class::get("NSUUID").unwrap();
            let obj: *mut Object = msg_send![class, alloc];
            let obj: *mut Object = msg_send![obj, initWithUUIDString: nsstring];
            Id::from_ptr(obj as *mut NSUUID)
        }
    }
}

unsafe impl objc::Message for NSUUID {}

impl INSObject for NSUUID {
    fn class() -> &'static Class {
        Class::get("NSUUID").unwrap()
    }
}
