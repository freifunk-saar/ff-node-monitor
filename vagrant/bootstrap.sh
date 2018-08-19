#!/usr/bin/env bash
# bootstrap script to install ff-node-monitor

set -xe
export DEBIAN_FRONTEND=noninteractive

if [ "$0" == "/tmp/vagrant-shell" ]; then
  cd /vagrant
else
  cd vagrant
fi

: "#### include default config or user config, that is in .gitignore:"
if [ -f vagrant.config ]; then
  source vagrant.config
else
  source vagrant.config.dist
fi

# some variables used in this script

# port and URL
PORT=8833 # matches the port forwarding in Vagrantfile
ROOT_URL="http://localhost:$PORT"

HOME_PATH='/opt/ff-node-monitor'

FFNM_USERNAME="ff-node-monitor"

: "#### We need some development libraries for the build process:"
sudo apt update && sudo apt -y install git curl gcc pkg-config libssl-dev libpq-dev ssmtp

: "#### setup locale"
sed -i 's/# \(\(de_DE\|en_US\)\.UTF-8 UTF-8\)/\1/' /etc/locale.gen && dpkg-reconfigure --frontend=noninteractive locales
cat /etc/locale.gen | fgrep UTF-8

: "#### create a user for this service, and change to its home directory:"
sudo adduser $FFNM_USERNAME --home "$HOME_PATH" --system
cd "$HOME_PATH"
sudo chown $FFNM_USERNAME .

ffsudo="sudo -u $FFNM_USERNAME"

: "#### fetch the ff-node-monitor sources:"
# do not fail when re-provisioning
test -d src || $ffsudo git clone https://github.com/freifunk-saar/ff-node-monitor.git src

: "#### ff-node-monitor is written in Rust using Rocket, which means it needs a nightly version of Rust:"
if ! test -f $HOME_PATH/.cargo/bin/rustc; then
    $ffsudo curl https://sh.rustup.rs -sSf -o rustup.sh
    $ffsudo sh rustup.sh -y --default-toolchain $(cat "$HOME_PATH/src/rust-version")
    $ffsudo rm rustup.sh
fi

: "#### build the ff-node-monitor "
cd "$HOME_PATH/src"
$ffsudo "$HOME_PATH/.cargo/bin/cargo" build --release

: "#### Database setup"
apt -y install postgresql
psql="sudo -u postgres psql"
if ! $psql -lqt | cut -d \| -f 1 | grep -qw ff-node-monitor; then
    $psql -c 'DROP ROLE IF EXISTS "'$FFNM_USERNAME'"; CREATE ROLE "'$FFNM_USERNAME'" WITH LOGIN;'
    $psql -c 'CREATE DATABASE "'$FFNM_USERNAME'" WITH OWNER = "'$FFNM_USERNAME'" LC_COLLATE = '\''de_DE.utf8'\'' TEMPLATE template0;'
fi

: "#### Service setup"
: "#### The service loads its configuration from a Rocket.toml file in the source directory. You can start by copying the template"
cd "$HOME_PATH/src"
$ffsudo touch Rocket.toml
$ffsudo chmod 777 Rocket.toml
cat <<EOF > Rocket.toml
[global.ff-node-monitor.ui]
# The name of your Freifunk community.
instance_name = "$INSTANCE_NAME"
# The sentence "Willkommen bei $instance_article_dative $instance_name" should be grammatically
# correct.
instance_article_dative = "der"
# The sender address of the emails that are sent by ff-node-monitor:
email_from = "$EMAIL_FROM"

[global.ff-node-monitor.urls]
# The root URL where you will be hosting ff-node-monitor.
root = "$ROOT_URL"
# The URL to the hopglass nodes.json file.
nodes = "$NODES_URL"
# URL to the source code (needed for AGPL compliance).  You can leave this unchanged if you didn't
# change the code.  Otherwise, you have to upload the changed code somewhere and point to it here.
sources = "https://github.com/freifunk-saar/ff-node-monitor"
# Optional: Absolute URL to another sytelsheet that is included in the page.
#stylesheet = "https://..."

[global.ff-node-monitor.secrets]
# PostgreSQL credentials.  If you followed the instructions in the README, this
# should be correct.
postgres_url = "postgres://$FFNM_USERNAME@/$FFNM_USERNAME"
# Key used to sign data for confirmation emails:
action_signing_key = "$(openssl rand -hex 32)"
# Optional: Host to submit emails to.  That host must accept email with arbitrary destination
# from this service.
smtp_host = "localhost"

[global]
# The address and port on which ff-node-monitor will listen.
address = "0.0.0.0"
port = $PORT
# Secret key used by Rocket:
secret_key = "$(openssl rand -base64 32)"
EOF
$ffsudo chmod 600 Rocket.toml

: "#### To run the service using systemd, the .service file needs to be installed:"
sudo cp ff-node-monitor.service /etc/systemd/system/
: "#### If you change the HOME_PATH, you also have to adapt the path in the service file"
sudo sed -i 's|/opt/ff-node-monitor|'"$HOME_PATH"'|g' /etc/systemd/system/ff-node-monitor.service
sudo systemctl daemon-reload
sudo systemctl stop ff-node-monitor
sudo systemctl enable ff-node-monitor.service
sudo systemctl start ff-node-monitor
sudo systemctl status ff-node-monitor

: "#### Finally, the service relies on a cron job to regularly check in on all the nodes and send notifications when their status changed:"
(sudo crontab -u $FFNM_USERNAME -l; echo "*/5 * * * *    curl $ROOT_URL/cron" ) | sudo crontab -u $FFNM_USERNAME -

: "#### read node data initially:"
sleep 1
$ffsudo curl $ROOT_URL/cron

echo "The site should now be reacheable under $ROOT_URL"
