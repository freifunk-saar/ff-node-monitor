#!/usr/bin/env bash
# bootstrap script to install ff-node-monitor

set -xe
export DEBIAN_FRONTEND=noninteractive

IS_VAGRANT=0
IS_TRAVIS=0
if [ "$0" == "/tmp/vagrant-shell" ]; then
  WORKING_DIR="/vagrant"
  IS_VAGRANT=1
else
  WORKING_DIR="`pwd`"/"`dirname "$0"`"
  if [ -z "${WORKING_DIR##*/home/travis/build/*}" ] ;then
    echo "The Workingdir '$WORKING_DIR' contains substring: '/home/travis/build/'."
    IS_TRAVIS=1
  fi
fi


: "#### include default config or user config, that is in .gitignore:"
if [ -f "$WORKING_DIR"/vagrant.config ]; then
  source "$WORKING_DIR"/vagrant.config
else
  source "$WORKING_DIR"/vagrant.config.dist
fi

# some variables used in this script

# URL the service will be available from the outside (external IP)
# this must match the actual IP handed out by Vagrant
# TODO: maybe determine that IP in the script somehow
EXTERNAL="10.19.0.2"

ROOT_URL="http://localhost:$PORT"

WEB_URL="http://$EXTERNAL:$PORT"

HOME_PATH='/opt/ff-node-monitor'

FFNM_USERNAME="ff-node-monitor"

: "#### We need some development libraries for the build process:"
sudo apt update && sudo apt -y install git curl gcc pkg-config libssl-dev libpq-dev ssmtp

: "#### create a user for this service, and change to its home directory:"
sudo adduser $FFNM_USERNAME --home "$HOME_PATH" --system
cd "$HOME_PATH"
sudo chown $FFNM_USERNAME .

ffsudo="sudo -u $FFNM_USERNAME"

: "#### fetch the ff-node-monitor sources:"
$ffsudo git clone https://github.com/freifunk-saar/ff-node-monitor.git src

: "#### ff-node-monitor is written in Rust using Rocket, which means it needs a nightly version of Rust:"
$ffsudo curl https://sh.rustup.rs -sSf -o rustup.sh
$ffsudo sh rustup.sh -y --default-toolchain $(cat "$HOME_PATH/src/rust-version")
$ffsudo rm rustup.sh

: "#### build the ff-node-monitor "
cd "$HOME_PATH/src"
$ffsudo "$HOME_PATH/.cargo/bin/cargo" build --release

: "#### Database setup"
apt -y install postgresql
sudo -u postgres psql -c 'CREATE ROLE "'$FFNM_USERNAME'" WITH LOGIN;' 
sudo -u postgres psql -c 'CREATE DATABASE "'$FFNM_USERNAME'" WITH OWNER = "'$FFNM_USERNAME'";'

: "#### Service setup"
: "#### The service loads its configuration from a Rocket.toml file in the source directory. You can start by copying the template"
cd "$HOME_PATH/src"
$ffsudo touch Rocket.toml
$ffsudo chmod 777 Rocket.toml
cat <<EOF > Rocket.toml
[global.ff-node-monitor.ui]
# The name of your Freifunk community.
instance_name = "$INSTANCE_NAME"
# The sender address of the emails that are sent by ff-node-monitor:
email_from = "$EMAIL_FROM"

[global.ff-node-monitor.urls]
# The root URL where you will be hosting ff-node-monitor.
root = "$WEB_URL"
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
sudo systemctl enable ff-node-monitor.service
sudo systemctl start ff-node-monitor
sudo systemctl status ff-node-monitor

: "#### Finally, the service relies on a cron job to regularly check in on all the nodes and send notifications when their status changed:"
(sudo crontab -u $FFNM_USERNAME -l; echo "*/5 * * * *    curl $ROOT_URL/cron" ) | sudo crontab -u $FFNM_USERNAME -

: "#### read node data initially:"
sleep 10
$ffsudo curl $ROOT_URL/cron

echo "The site should now be reacheable under $WEB_URL"
