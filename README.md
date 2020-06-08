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

Make sure you have at least 1.5 GB free disk space.

### Build Process

1.  First, let's create a user for this service, and change to its home directory:

    ```
    sudo adduser ff-node-monitor --home /opt/ff-node-monitor --system
    cd /opt/ff-node-monitor
    ```

2.  We need some development libraries for the build process:

    ```
    sudo apt install libssl-dev libpq-dev libc6-dev curl gcc pkg-config
    ```

3.  *ff-node-monitor* is written in [Rust](https://www.rust-lang.org/) using
    [Rocket](https://rocket.rs/), which means it needs a nightly version of Rust:

    ```
    curl https://sh.rustup.rs -sSf > rustup.sh
    sudo -u ff-node-monitor sh rustup.sh --default-toolchain $(cat rust-version)
    rm rustup.sh
    ```

    The file `rust-version` always contains a tested nightly version number. If
    you want the latest nightly version instead, just use `--default-toolchain nightly` .

4.  Now we can fetch the sources and build them:

    ```
    sudo -u ff-node-monitor git clone https://github.com/freifunk-saar/ff-node-monitor.git src
    cd src
    sudo -u ff-node-monitor /opt/ff-node-monitor/.cargo/bin/cargo build --release
    ```

    > The build process takes a while, you can already finish the Database Setup
    > (steps 6. and 7.) and part of the Service Setup (step 8. and 10.) in a
    > second shell as the build process continues.

5.  This step is optional, but if you want to save some disk space, you can now
    clean up the build directory:

    ```
    rm -rf target/release/{build,deps,incremental,.fingerprint}
    ```

    Over time, you will also accumulate more and more different Rust versions.
    You can use

    ```
    sudo -u ff-node-monitor ~ff-node-monitor/.cargo/bin/rustup toolchain list
    ```

    to see which versions you have installed, and then `toolchain uninstall`
    the ones you do not need any more (all but the last, most likely).

### Database setup

6.  *ff-node-monitor* needs PostgreSQL as a database backend:

    ```
    sudo apt install postgresql
    ```

7.  We will use the `ff-node-monitor` system user to access PostgreSQL, and we
    need to create a database for the service:

    ```
    sudo -u postgres psql -c 'CREATE ROLE "ff-node-monitor" WITH LOGIN;'
    sudo -u postgres psql -c 'CREATE DATABASE "ff-node-monitor" WITH OWNER = "ff-node-monitor" LC_COLLATE = '\''de_DE.utf8'\'' TEMPLATE template0;'
    ```

    You may have to install the `de_DE.UTF-8` locale before this works.  On
    Debian, run `sudo dpkg-reconfigure locales` to do so.

### Service setup

8.  The service loads its configuration from a `Rocket.toml` file in the source
    directory.  You can start by copying the template:

    ```
    cd /opt/ff-node-monitor/src
    sudo -u ff-node-monitor cp Rocket.toml.dist Rocket.toml
    chmod 600 Rocket.toml
    ```

    Most of the values in your `Rocket.toml` will need to be changed; see the comments in the
    template for what to do and how.

9.  To run the service using systemd, the `.service` file needs to be installed:

    ```
    sudo cp ff-node-monitor.service /etc/systemd/system/
    sudo systemctl daemon-reload
    sudo systemctl enable ff-node-monitor
    sudo systemctl start ff-node-monitor
    sudo systemctl status ff-node-monitor
    ```

    If the last command does not show the service as running, you need to debug
    and fix whatever issue has come up.

10. To expose the service on the internet, set up a reverse proxy in your main
    web server. If you are not already running a web server, `nginx` is a good
    choice.  You will have to edit your site configuration, usually located at
    `/etc/nginx/sites-enabled/default`.  Here's the necessary snippet, mounting
    the node monitor in the `node-monitor` subdirectory:

    ```
    location /node-monitor/ {
        proxy_pass http://127.0.0.1:8833/;
    }
    # Directly serve static files, no need to run them through the app
    location /node-monitor/static/ {
        alias /opt/ff-node-monitor/src/static/;
    }
    ```

    Test your configuration and reload nginx:

    ```
    nginx -t
    service nginx reload
    ```

    Now, accessing the service at whatever `root` URL you configured in the
    `Rocket.toml` should work.

11. Finally, the service relies on a cron job to regularly check in on all the
    nodes and send notifications when their status changed:

    ```
    sudo crontab -e -u ff-node-monitor
    ```

    Add the following line to that crontab, replacing `$ROOT_URL` by your `root` URL
    (as configured in `Rocket.toml`):

    ```
    */5 * * * *    curl -s $ROOT_URL/cron
    ```

That's it!  The service should now be running and working.

## Upgrade

To upgrade the service to the latest git version, follow these steps:

```
cd /opt/ff-node-monitor/src/
git pull
sudo rm target/release/ff-node-monitor
sudo -u ff-node-monitor /opt/ff-node-monitor/.cargo/bin/rustup default $(cat rust-version)
sudo -u ff-node-monitor /opt/ff-node-monitor/.cargo/bin/cargo build --release
sudo systemctl restart ff-node-monitor
```

Check [the CHANGELOG](CHANGELOG.md) to see if any manual steps are needed.

## Debugging

When something goes wrong, the first step should be to look at the error log:

```
sudo journalctl -u ff-node-monitor.service
```

## Customization

If you want to adapt the node monitor to the layout of your web presence, you
can set `stylesheet` to an external CSS file in your `Rocket.toml`.
Put any images and other static data into `src/static/`. Here a CSS-example:

```
#title {
    padding-top: 131px;
    background: url('static/logo.png');
    background-repeat: no-repeat;
}
```

## Development Virtual Environment

You can easily set up a test VM using Vagrant.

If you want to tweak the default configuration (which you do not have to), first
copy the default config file `vagrant.config.dist` to `vagrant.config` and
customize the configuration. Then follow these steps:

```
sudo apt install vagrant
cd vagrant
vagrant up ff-service
```

This will take a while, downloading the Vagrant box and install a running system
inside. You can adapt `bootstrap.sh` as you like to test around with different
settings. In your real setup you at least have to change the root URL where you
will be hosting ff-node-monitor.

You can then access the vagrant box at `http://localhost:8833`. If you want to
login the server use

```
vagrant ssh ff-service
```

If you want to delete and start over use

```
vagrant destroy ff-service
vagrant up ff-service
```

If you want to send out emails, one easy option is [msmtp](https://wiki.debian.org/msmtp).

For gmail (with deactivated 2-factor login) use this configuration in `/etc/msmtprc`:

```
# Set default values for all following accounts.
defaults
port 587
tls on
tls_trust_file /etc/ssl/certs/ca-certificates.crt

account gmail
host smtp.gmail.com
from <user>@gmail.com
auth on
user <user>
password <your password>

# Set a default account
account default : gmail
```

To test it run

```
echo -e "Subject: msmtp test\nhello test." | msmtp _recipient_address_
```

You should find your sent e-mail in the recipient's inbox shortly afterwards.

### vagrant troubleshooting

In case the install in the mashine fails, consider to upgrade your vagrant box with

```
vagrant box update
```
