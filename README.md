# iMonitor (projet personnel)

Device monitoring through remote lockdownd commands.

## Disclaimer

This is a personal project, maintained when I have time.
This work is heavily based on the wonderful idevice crate by Jackson Coxson (https://github.com/jkcoxson/idevice).

Do not consider this stable, some features are still under development, mainly :
- Unified logs archive extraction
- Time coverage

## Code structure

- `imonitor-lib`: library that implements all the logic (service connection, data retrieval, etc.)
- `imonitor`: main binary depending on `imonitor-lib`. This only wraps configuration to monitor multiple devices.
- `imonitor-enroll`: enroll the device for monitoring (extract a pairing file + set EnableWifiConnections in lockdown).
- `logger`: custom logging macro to facilitate logging to the right device file, based on the Rust context. Used in `imonitor-lib`.

## Documentation

See `documentation` directory.

## Useful links

- `https://github.com/doronz88/pymobiledevice3/blob/master/misc/understanding_idevice_protocol_layers.md`
- `https://github.com/danielpaulus/go-ios/blob/main/usbmuxdbuild/Dockerfile`
