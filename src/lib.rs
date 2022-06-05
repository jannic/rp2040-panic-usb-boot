#![no_std]

use core::fmt::Write;
use core::panic::PanicInfo;
use rp2040_hal::pac as rp2040;

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
    let p = unsafe { rp2040::Peripherals::steal() };
    // disable XIP cache so cache ram becomes usable
    p.XIP_CTRL
        .ctrl
        .write(|w| w.power_down().clear_bit().en().clear_bit());

    // write panic message to XIP RAM
    let buf: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(0x15000000 as *mut u8, 0x4000) };
    let mut cur = Cursor::new(buf);
    write!(&mut cur, "{}\n\0", info).ok();

    // For usb_boot to work, XOSC needs to be running
    if !(p.XOSC.status.read().stable().bit()) {
        p.XOSC.startup.write(|w| unsafe {
            w.delay().bits((12_000 /*kHz*/ + 128) / 256)
        });
        p.XOSC.ctrl.write(|w| {
            w.freq_range()
                .variant(rp2040::xosc::ctrl::FREQ_RANGE_A::_1_15MHZ)
                .enable()
                .variant(rp2040::xosc::ctrl::ENABLE_A::ENABLE)
        });
        while !(p.XOSC.status.read().stable().bit()) {}
    }

    // jump to usb
    rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
    loop {}
}
