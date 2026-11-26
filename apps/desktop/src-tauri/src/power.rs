use std::sync::atomic::{AtomicU64, Ordering};

// The low bit is the suspended flag. The upper bits count real transitions.
// Keeping these together lets the scheduler detect a full suspend/resume pair
// between samples. Duplicate resume notifications leave the same stamp intact.
static POWER_STAMP: AtomicU64 = AtomicU64::new(0);

pub fn stamp() -> u64 {
    POWER_STAMP.load(Ordering::Acquire)
}

fn set_suspended(suspended: bool) {
    let flag = u64::from(suspended);
    let _ = POWER_STAMP.try_update(Ordering::AcqRel, Ordering::Acquire, |old| {
        (old & 1 != flag).then_some((old.wrapping_add(2) & !1) | flag)
    });
}

#[cfg(windows)]
mod platform {
    use super::set_suspended;
    use std::ffi::c_void;
    use std::ptr::null_mut;

    const DEVICE_NOTIFY_CALLBACK: u32 = 2;
    const PBT_APMSUSPEND: u32 = 0x0004;
    const PBT_APMRESUMESUSPEND: u32 = 0x0007;
    const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;

    #[repr(C)]
    struct DeviceNotifySubscribeParameters {
        callback: unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> u32,
        context: *mut c_void,
    }

    #[link(name = "Kernel32")]
    extern "system" {
        fn QueryUnbiasedInterruptTimePrecise(unbiased_time: *mut u64);
    }

    #[link(name = "PowrProf")]
    extern "system" {
        fn PowerRegisterSuspendResumeNotification(
            flags: u32,
            recipient: *mut c_void,
            registration_handle: *mut *mut c_void,
        ) -> u32;
        fn PowerUnregisterSuspendResumeNotification(registration_handle: *mut c_void) -> u32;
    }

    pub fn active_time_100ns() -> u64 {
        let mut active = 0;
        // SAFETY: the Windows API writes one ULONGLONG into this valid local slot.
        unsafe { QueryUnbiasedInterruptTimePrecise(&mut active) };
        active
    }

    unsafe extern "system" fn callback(_: *mut c_void, event: u32, _: *mut c_void) -> u32 {
        match event {
            PBT_APMSUSPEND => set_suspended(true),
            PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC => set_suspended(false),
            _ => {}
        }
        0
    }

    pub struct Registration {
        handle: *mut c_void,
        // Preserve the registration data for the lifetime of this handle.
        _subscription: Box<DeviceNotifySubscribeParameters>,
    }

    impl Registration {
        pub fn new() -> Result<Self, u32> {
            let mut subscription = Box::new(DeviceNotifySubscribeParameters {
                callback,
                context: null_mut(),
            });
            let mut handle = null_mut();
            // SAFETY: the callback is a process-lifetime function using only a static
            // atomic, subscription has the documented ABI, and handle is an out slot.
            let result = unsafe {
                PowerRegisterSuspendResumeNotification(
                    DEVICE_NOTIFY_CALLBACK,
                    (&mut *subscription as *mut DeviceNotifySubscribeParameters).cast(),
                    &mut handle,
                )
            };
            if result == 0 {
                Ok(Self { handle, _subscription: subscription })
            } else {
                Err(result)
            }
        }
    }

    impl Drop for Registration {
        fn drop(&mut self) {
            // SAFETY: only this guard owns the successful registration handle.
            unsafe { PowerUnregisterSuspendResumeNotification(self.handle) };
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::sync::OnceLock;
    use std::time::Instant;

    pub fn active_time_100ns() -> u64 {
        static START: OnceLock<Instant> = OnceLock::new();
        (START.get_or_init(Instant::now).elapsed().as_nanos() / 100) as u64
    }

    pub struct Registration;

    impl Registration {
        pub fn new() -> Result<Self, u32> {
            Err(50)
        }
    }
}

pub use platform::{active_time_100ns, Registration};
