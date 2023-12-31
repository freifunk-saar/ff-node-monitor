## 2023-12-31

* We updated to Rocket v0.5. This is almost entirely an internal change, but it has two user-visible consequences:
  The `ROCKET_ENV` environment variable no longer has any effect. You should remove it from your systemd service file.
  (There is a new `ROCKET_PROFILE` environment variable with a similar effect, but it should not be needed
  for usual deployments. Just make sure you build in release mode, i.e., `cargo build --release` or `cargo install`).
  Furthermore, you can now use stable Rust to build ff-node-monitor; a nightly version is no longer required.

## 2018-12-10

* We switched to Rocket's built-in support for managing DB connection pools.  You have to **update your `Rocket.toml`**:
  Remove `postgres_url = "postgres://ff-node-monitor@/ff-node-monitor"` from the `[global.ff-node-monitor.secrets]` section, and add `postgres = { url = "postgres://ff-node-monitor@/ff-node-monitor" }` to the `[global.databases]` section.
