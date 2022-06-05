# Reboot to USB mode on panic

On panic, the USB boot mode implemented in ROM will be
called, providing access for UF2 uploads and `picotool`.

# Usage

Just add this to your `main.rs`:

```
use rp2040_panic_usb_boot as _;
```

# Panic messages

Additionally, panic messages are written to the XIP RAM,
after disabling XIP caching.

That way, the panic message can be read using picotool:

```
picotool save -r 0x15000000 0x15004000 message.bin
hexdump -C message.bin
```

RAM contents can be read the same way, by reading from
address `0x20000000`.
