## 2018-12-10

* We switched to Rocket's built-in support for managing DB connection pools.  You have to **update your `Rocket.toml`**:
  Remove `postgres_url = "postgres://ff-node-monitor@/ff-node-monitor"` from the `[global.ff-node-monitor.secrets]` section, and add `postgres = { url = "postgres://ff-node-monitor@/ff-node-monitor" }` to the `[global.databases]` section.
