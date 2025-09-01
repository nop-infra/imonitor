## Done
- Implement retrying heartbeat connect when services fail too many times``
  - Handle connection reset by peer for VPN disconnect
- Makes monitor tasks parallel
- Crash files : manage directories + sysdiagnose
- Fix crash files cleaning
- Make a lib
- Implement monitoring multiple devices
    - config file dedicated to device list
- Implement persistence for heartbeat last date
- Persist known directories
- Fix disappeared file that cannot be downloaded (see TODO in client file)
- Implement binary to enroll device
- Manage heartbeat with channel messages
- Move pairing file to connection directory
- Logging file by monitored device
  - To test with multiple devices
- Remove timeout for syslog (cf. TODO in code)
- Create systemd unit
- Remove label suffix (SecOPS)
- Get old logs with OsTraceRelay (bitmap of holes in monitoring)
- [x] Use OsTraceRelay for syslog
  - [x] Compare with syslog relay
  - [x] Write JSON instead of debug form
  - [x] Ask idevice maintainer to derive Serialize for OsTraceLog and nested structs
- Use base_dir from config

## To finish
- Persist covered_activity to fs
  - Check why first recorded period is not written

## TODO

- Fix activity_coverage : when log start streaming but get back online without heartbeat (at most RECONNECT_WAITING_TIME lost but not in gaps)
- Fix os trace archive
  - Maximum size not working
  - Clean up activity coverage to prevent asking for old periods no more in the logs.
    - The dates older than first log of last retrieved archive sould be discarded
  - Set maximum size of unified logs retrieved -> not working
    - Get last log and insert corresponding covered range
- Handle config/monitored devices path as CLI parameters
- Harmonize refresh rates and use configuration values
- Generate new pairing file at each connection
    - Handle error "device does not have pairing file" -> needs new pairing file, exit (or reload config)
- Inotify for config / monitored devices
- Fix known crash files/dirs cleaning (find a way to do it cleanly)
  - Idea : if files_to_get.len() = 0 multiple times, clean it
  - Other idea : set variable when dir listing is over
- Implement binary that send progressively to S3 (setup priority in systemd unit)
- Implement device info to save in device struct
- Get process list (only pid list with OsTraceRelay, or use dvt in dev. mode. Compare with pymobiledevice, maybe name)

# Optional

- Use notification listening to check for lockdown alerts / incoming message
- Use instproxy to list apps
- Use diagnostics to generate sysdiagnose (dev mode required)
  - Check if keyboard shortcut can be emulated
- Try using tunnel for speed improvment
- Search remote xpc services capabilities
- Install VPN profile via misagent / use headscale
- Use CoreDeviceProxy, check RemoteXPC services
