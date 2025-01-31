#![no_std]

use core::fmt::Write;
use core::panic::PanicInfo;

struct Cursor<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a mut [u8]) -> Cursor<'a> {
        Cursor { buf, pos: 0 }
    }
}

impl<'a> core::fmt::Write for Cursor<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let len = s.as_bytes().len();
        if len < self.buf.len() - self.pos {
            self.buf[self.pos..self.pos + len].clone_from_slice(s.as_bytes());
            self.pos += len;
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    // disable XIP cache so cache ram becomes usable
    disable_xip_cache();

    // write panic message to XIP RAM
    let buf: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(0x15000000 as *mut u8, 0x4000) };
    let written = {
        let mut cur = Cursor::new(buf);
        write!(&mut cur, "{}\n\0", info).ok();
        cur.pos
    };

    // Clear the rest of XIP RAM so it's not full of garbage when dumped
    buf[written..0x4000].fill(0);

    // For usb_boot to work, XOSC needs to be running
    if !xosc_is_running() {
        xosc_start_delay((12_000 /*kHz*/ + 128) / 256);
        xosc_enable(true);
        while !(xosc_is_running()) {}
    }

    // jump to usb. the unwrap here should never occur unless ROM is faulty.
    (ROMFuncs::load().unwrap().reset_to_usb_boot)(0, 0);
    loop {}
}

// find_func and ROMFuncs impls borrowed from rp-rs/flash-algo
// used here instead of rp2040-hal romfuncs to avoid coupling this crate to a specific hal
fn find_func<T>(tag: [u8; 2]) -> Option<T> {
    let tag = u16::from_le_bytes(tag) as u32;
    type RomTableLookupFn = unsafe extern "C" fn(table: *const u16, code: u32) -> usize;
    /// This location in flash holds a 16-bit truncated pointer for the ROM lookup function
    const ROM_TABLE_LOOKUP_PTR: *const u16 = 0x0000_0018 as _;
    /// This location in flash holds a 16-bit truncated pointer for the ROM function table
    /// (there's also a ROM data table which we don't need)
    const FUNC_TABLE: *const u16 = 0x0000_0014 as _;
    unsafe {
        let lookup_func = ROM_TABLE_LOOKUP_PTR.read() as usize;
        let lookup_func: RomTableLookupFn = core::mem::transmute(lookup_func);
        let table = FUNC_TABLE.read() as usize;
        let result = lookup_func(table as *const u16, tag);
        if result == 0 {
            return None;
        }
        Some(core::mem::transmute_copy(&result))
    }
}

struct ROMFuncs {
    reset_to_usb_boot: extern "C" fn(gpio_activity_pin_mask: u32, disable_interface_mask: u32),
}

impl ROMFuncs {
    fn load() -> Option<Self> {
        Some(ROMFuncs {
            reset_to_usb_boot: find_func(*b"UB")?,
        })
    }
}

// implement basic register access layer to avoid depending on a PAC
// we only need a few registers, it's not so bad to write it by hand

struct Reg {
    address: *mut u32,
}

impl Reg {
    const fn new(address: u32) -> Self {
        Self {
            address: address as *mut u32,
        }
    }

    fn read(&self) -> u32 {
        unsafe { self.address.read_volatile() }
    }

    fn write(&self, value: u32) {
        unsafe {
            self.address.write_volatile(value);
        }
    }
}

/// XIP_CTRL
///
/// bit 3 POWER_DOWN - when 1, cache is powered down - it retains state but cannot be accessed.
/// bit 1 ERR_BADWRITE - when 1, writes to any alias other than 0x0 will produce a bus fault
/// bit 1 EN - when 1, enable the cache.
const XIP_CTRL: Reg = Reg::new(0x1400_0000);

/// XOSC_CTRL
///
/// 23:12 ENABLE - on powerup this field is initialsed to DISABLE and the chip runs from the ROSC
///                Enumerated values: 0xd1e -> DISABLE, 0xfab -> ENABLE
/// 11:0 FREQ_RANGE: Frequency range. This resets to 0xAA0 and cannot be changed.
const XOSC_CTRL: Reg = Reg::new(0x4002_4000);

/// XOSC: STATUS Register
///
/// 31:31 STABLE - Oscillator is running and stable
/// 24:24 BADWRITE - An invalid value has been written to CTRL_ENABLE
/// 12:12 ENABLED - Oscillator is enabled but not necessarily running and stable
/// 1:0 FREQ_RANGE - The current frequency range, always reads 0
const XOSC_STATUS: Reg = Reg::new(0x4002_4004);

/// XOSC: STARTUP Register
///
/// 20:20 X4: Multiplies the startup_delay by 4
/// 13:0 DELAY: in multiples of 256*xtal_period.
const XOSC_STARTUP: Reg = Reg::new(0x4002_400c);

// helper functions to make logicavoid direct register access in panic handler

fn disable_xip_cache() {
    // not POWER_DOWN, not ERR_BADWRITE, not EN
    XIP_CTRL.write(0);
}

fn xosc_is_running() -> bool {
    // return true if STABLE bit is set
    (XOSC_STATUS.read() & (1 << 31)) == (1 << 31)
}

fn xosc_start_delay(delay: u32) {
    // delay is the low 14 bits (13:0) of the register.
    debug_assert!(delay < (1 << 14));
    let delay = delay & (1 << 14);
    XOSC_STARTUP.write(delay);
}

fn xosc_enable(enable: bool) {
    // There's only one valid frequency range, so set that
    let freq_range = 0xaa0;
    let enable_val = match enable {
        true => 0xfab,
        false => 0xd1e,
    };
    XOSC_CTRL.write(freq_range | (enable_val << 12));
}
