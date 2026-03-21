# Setup

The following steps must be followed to setup the tool:

- Drop imonitor binary on the server
- Install the systemd unit (`example/systemd/imonitor.service`)
- Install and configure a VPN on the server and on the monitored phone (Wireguard / OpenVPN / IPSec / what you like)
- Enroll the device with `imonitor-enroll`
- Drop main imonitor config (`config.toml`) in imonitor working directory (can be setup in systemd unit file)
  - An example is ready in `example/config.toml`
- Drop monitored devices config (`devices.toml`) in imonitor working directory.
  - An example is in `example/devices/devices.toml`
- Start systemd unit
- Enjoy
