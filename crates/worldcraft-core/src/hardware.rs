//! Physical memory, used for conservative admission and work budgets, never an allocation guarantee.
#[derive(Clone, Copy, Debug, Default)]
pub struct Memory {
    pub total: u64,
    pub available: u64,
}
pub const GIB: u64 = 1024 * 1024 * 1024;
pub fn memory() -> Option<Memory> {
    #[cfg(target_os = "linux")]
    {
        parse_meminfo(&std::fs::read_to_string("/proc/meminfo").ok()?)
    }
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Status {
            length: u32,
            load: u32,
            total: u64,
            available: u64,
            page_total: u64,
            page_available: u64,
            virtual_total: u64,
            virtual_available: u64,
            extended: u64,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GlobalMemoryStatusEx(status: *mut Status) -> i32;
        }
        let mut status = Status {
            length: std::mem::size_of::<Status>() as u32,
            load: 0,
            total: 0,
            available: 0,
            page_total: 0,
            page_available: 0,
            virtual_total: 0,
            virtual_available: 0,
            extended: 0,
        };
        // SAFETY: correctly sized, initialized MEMORYSTATUSEX, valid for this call.
        (unsafe { GlobalMemoryStatusEx(&mut status) } != 0).then_some(Memory {
            total: status.total,
            available: status.available,
        })
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}
#[cfg(any(target_os = "linux", test))]
fn parse_meminfo(text: &str) -> Option<Memory> {
    let read = |key: &str| -> Option<u64> {
        text.lines().find_map(|line| {
            let value = line.strip_prefix(key)?.split_whitespace().next()?;
            value.parse::<u64>().ok()?.checked_mul(1024)
        })
    };
    Some(Memory {
        total: read("MemTotal:")?,
        available: read("MemAvailable:")?,
    })
}
pub fn recommend_eco(threads: usize, memory: Option<Memory>) -> bool {
    threads <= 4 || memory.is_none_or(|m| m.total < 9 * GIB || m.available < 3 * GIB)
}
pub fn detailed_admission(memory: Option<Memory>) -> Result<(), String> {
    if memory.is_none_or(|m| m.available < 6 * GIB) {
        return Err("Detailed AI generation needs at least 6 GiB of available system RAM. Use Fast regional or an offline example, or close other apps first.".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservative_detection_and_admission() {
        let ample = Some(Memory {
            total: 32 * GIB,
            available: 20 * GIB,
        });
        assert!(!recommend_eco(8, ample));
        assert!(recommend_eco(4, ample));
        assert!(recommend_eco(16, None));
        assert!(detailed_admission(None).is_err());
        assert!(detailed_admission(Some(Memory {
            total: 16 * GIB,
            available: 5 * GIB
        }))
        .is_err());
        assert!(detailed_admission(ample).is_ok());
    }
    #[test]
    fn linux_memory_uses_available_not_just_free() {
        let m = parse_meminfo("MemTotal: 8388608 kB\nMemFree: 100 kB\nMemAvailable: 3145728 kB\n")
            .unwrap();
        assert_eq!(m.total, 8 * GIB);
        assert_eq!(m.available, 3 * GIB);
        assert!(parse_meminfo("MemTotal: nonsense").is_none());
    }
}
