# ff-node-monitor [![Build Status](https://travis-ci.org/freifunk-saar/ff-node-monitor.svg?branch=master)](https://travis-ci.org/freifunk-saar/ff-node-monitor)

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

1.  First, let's create a user for this service, and change to its home directory:

    ```
    sudo adduser ff-node-monitor --home /opt/ff-node-monitor --system
    cd /opt/ff-node-monitor
    ```

2.  We need some development libraries for the build process:

    ```
    sudo apt install libssl-dev libpq-dev curl build-essential pkg-config
    ```

3.  *ff-node-monitor* is written in [Rust](https://www.rust-lang.org/) using
    [Rocket](https://rocket.rs/), which means it needs a nightly version of Rust:

    ```
    curl https://sh.rustup.rs -sSf > rustup.sh
    sudo -u ff-node-monitor sh rustup.sh --default-toolchain nightly
    rm rustup.sh
    ```

4.  Now we can fetch the sources and build them:

    ```
    sudo -u ff-node-monitor git clone https://github.com/freifunk-saar/ff-node-monitor.git src
    cd src
    sudo -u ff-node-monitor /opt/ff-node-monitor/.cargo/bin/cargo build --release
    ```

    If that fails, it is possible that the latest Rust nightly is incompatible
    with one of the dependencies.  You can install and use a tested version
    using:

    ```
    sudo -u ff-node-monitor /opt/ff-node-monitor/.cargo/bin/rustup default $(cat rust-version)
    ```

### Database setup

1.  *ff-node-monitor* needs PostgreSQL as a database backend:

    ```
    sudo apt install postgresql
    ```

2.  We will use the `ff-node-monitor` system user to access PostgreSQL, and we
    need to create a database for the service:

    ```
    sudo -u postgres psql -c 'CREATE ROLE "ff-node-monitor" WITH LOGIN;'
    sudo -u postgres psql -c 'CREATE DATABASE "ff-node-monitor" WITH OWNER = "ff-node-monitor" LC_COLLATE = '\''de_DE.utf8'\'' TEMPLATE template0;'
    ```

    You may have to install the `de_DE.UTF-8` locale before this works.  On
    Debian, run `sudo dpkg-reconfigure locales` to do so.

### Service setup

1.  The service loads its configuration from a `Rocket.toml` file in the source
    directory.  You can start by copying the template:

    ```
    cd /opt/ff-node-monitor/src
    sudo -u ff-node-monitor cp Rocket.toml.dist Rocket.toml
    chmod 600 Rocket.toml
    ```

    Most of the values in there will need to be changed; see the comments in the
    template for what to do and how.

2.  To run the service using systemd, the `.service` file needs to be installed:

    ```
    sudo cp ff-node-monitor.service /etc/systemd/system/
    sudo systemctl daemon-reload
    sudo systemctl enable ff-node-monitor
    sudo systemctl start ff-node-monitor
    sudo systemctl status ff-node-monitor
    ```

    If the last command does not show the service as running, you need to debug
    and fix whatever issue has come up.

3.  To expose the service on the internet, set up a reverse proxy in your main
    web server.  Here's how that could look like for nginx (this is a snippet of
    the site configuration), using the `node-monitor` subdirectory:

    ```
    location /node-monitor/ {
        proxy_pass http://127.0.0.1:8833/;
    }
    # Directly serve static files, no need to run them through the app
    location /node-monitor/static/ {
        alias /opt/ff-node-monitor/src/static/;
    }
    ```

    Now, accessing the service at whatever `root` URL you configured in the
    `Rocket.toml` should work.

4.  Finally, the service relies on a cron job to regularly check in on all the
    nodes and send notifications when their status changed:

    ```
    sudo crontab -e -u ff-node-monitor
    ```

    Add the following line to that crontab, replacing `$ROOT_URL` by your `root` URL
    (as configured in `Rocket.toml`):

    ```
    */5 * * * *    curl $ROOT_URL/cron
    ```

That's it!  The service should now be running and working.

## Upgrade

Upgrade the service to the latest git version with these steps:

```
cd /opt/ff-node-monitor/src/
git pull
sudo rm target/release/ff-node-monitor
sudo -u ff-node-monitor /opt/ff-node-monitor/.cargo/bin/rustup default $(cat rust-version)
sudo -u ff-node-monitor /opt/ff-node-monitor/.cargo/bin/cargo build --release
sudo systemctl restart ff-node-monitor
```

Make sure to check `Rocket.toml.dist` for new mandatory config options and adapt
your config if necessary.

## Customization

If you want to adapt the node monitor to the layout of your web presence, you
can set `stylesheet` to an external CSS file in your `Rocket.toml`.

## Development Virtual Environment

You can easily set up a test VM using Vagrant.

Optionally first you can copy the default config file `vagrant.config.dist` to
`vagrant.config` and customize the configuration. Then follow these steps:

```
sudo apt install vagrant
cd vagrant
vagrant up ff-service
```

This will take a while, downloading the Vagrant box and install a running system
inside. You can adapt `bootstrap.sh` as you like to test around with different
settings. In your real setup you have to change the root URL where you will be
hosting ff-node-monitor.

You can then access the virtual server frontend under
http://10.19.0.2:8833. If you want to login the server use

```
vagrant ssh ff-service
```
If you want to send out emails configure `/etc/ssmtp/ssmtp.conf`.

