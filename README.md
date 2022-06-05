# Reboot to USB mode on panic

On panic, the USB boot mode implemented in ROM will be
called, providing access for UF2 uploads and `picotool`.

# Usage

Just add this to your `main.rs`:

```
use rp2040_panic_usb_boot as _;
```

# Panic messages

Before rebooting, XIP caching is disabled and panic messages
are written to the XIP RAM.

That way, the panic message can be read using picotool, eg.:

```
picotool save -r 0x15000000 0x15004000 message.bin
strings message.bin | head
```

RAM contents can be read the same way, by reading from
address `0x20000000`.

# License

The contents of this repository are dual-licensed under the _MIT OR Apache
2.0_ License. That means you can choose either the MIT license or the
Apache-2.0 license when you re-use this code. See `MIT` or `APACHE2.0` for more
information on each specific license.

Any submissions to this project (e.g. as Pull Requests) must be made available
under these terms.
