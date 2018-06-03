# ff-node-monitor

This is a simple web service for Freifunk networks that lets node operators
register to monitor their nodes.  It uses the `nodes.json` from
[hopglass](https://github.com/hopglass/hopglass) to detect which nodes are
online, and sends notifications when the online status changes.

## Setup

The setup consists of three parts: Getting the service built, getting the
database set up, and configuring the service to be run as a daemon.

I have tested the following steps on a Debian Stretch system; if you are using a
different version or a different distribution, you might have to change some of
the steps accordingly.

### Build Process

1. First, let's create a user for this service, and change to its home directory

```
sudo adduser ff-node-monitor --home /var/lib/ff-node-monitor --system
cd /var/lib/ff-node-monitor
```

2. We need some development library for the build process

```
sudo apt install libssl-dev libpq-dev
```

3. *ff-node-monitor* is written in [Rust](https://www.rust-lang.org/) using
[Rocket](https://rocket.rs/), which means it needs a nightly version of Rust.

```
curl https://sh.rustup.rs -sSf > rustup.sh
sudo -u ff-node-monitor sh rustup.sh -y --default-toolchain nightly
rm rustup.sh
```

4. Now we can fetch the sources and build them

```
git clone https://github.com/freifunk-saar/ff-node-monitor.git src
cd src
cargo build --release
```

### Database setup

### Service setup
