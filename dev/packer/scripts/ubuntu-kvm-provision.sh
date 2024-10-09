#!/bin/bash
set -o errexit
set -o nounset



# Install build tools
apt update 
apt-get install build-essential pkg-config git libssl-dev -y

wget https://apt.llvm.org/llvm.sh 
chmod +x ./llvm.sh
./llvm.sh 17 all
ln -s /usr/lib/llvm-17/bin/clang-cl /usr/bin/clang-cl
ln -s /usr/lib/llvm-17/bin/llvm-lib /usr/bin/llvm-lib
ln -s /usr/lib/llvm-17/bin/lld-link /usr/bin/lld-link
ln -s /usr/lib/llvm-17/bin/llvm-ml /usr/bin/llvm-ml

# Install rust toolchain
curl --proto '=https' --tlsv1.2 --retry 10 --retry-connrefused --location --silent --show-error --fail "https://sh.rustup.rs" | sh -s -- --default-toolchain none -y
. "$HOME/.cargo/env"
rustup toolchain install 1.81.0 
rustup target add x86_64-pc-windows-msvc # needed for building the guest binaries
rustup toolchain install nightly-2023-11-28-x86_64-unknown-linux-gnu # needed for fuzzing workflows
rustup component add rustfmt clippy

# Install cargo components
cargo install just minver_rs cargo-workspaces cargo-fuzz

# Instal azure-cli
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash

# Install github cli
(type -p wget >/dev/null || ( apt update &&  apt-get install wget -y)) \
&&  mkdir -p -m 755 /etc/apt/keyrings \
&& wget -qO- https://cli.github.com/packages/githubcli-archive-keyring.gpg |  tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null \
&&  chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
&& echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" |  tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
&&  apt update \
&&  apt install gh -y

# Install dotnet so the 1es hosted agent can run
# dotnet-sdk-6.0 is not available in default feeds for 24.04
# https://learn.microsoft.com/en-us/dotnet/core/install/linux-ubuntu#ubuntu-net-backports-package-repository
add-apt-repository ppa:dotnet/backports -y
apt install -y dotnet-sdk-6.0

# Install KVM
lscpu
sudo apt install -y qemu-kvm libvirt-daemon-system libvirt-clients virt-manager
ls -al /dev/kvm || true