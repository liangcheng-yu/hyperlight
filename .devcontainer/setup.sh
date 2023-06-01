#!/bin/bash

# Install dependencies
apt-get update
apt-get install -y \
    wget \
    unzip \
    curl \
    git \
    cmake \
    ninja-build \
    build-essential \
    dotnet-sdk-7.0

# Install flatc
wget https://github.com/google/flatbuffers/releases/download/v23.3.3/Linux.flatc.binary.clang++-12.zip
unzip Linux.flatc.binary.clang++-12.zip
mv ./flatc /usr/bin/flatc
rm -rf Linux.flatc.binary.clang++-12.zip
rm -rf flatc

# Install rust
curl https://sh.rustup.rs -sSf | sh -s -- -y

# Install just
curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to /usr/bin/

# Install flatcc
git clone https://github.com/dvidelabs/flatcc.git
flatcc/scripts/build.sh
chmod +x ./flatcc/bin/flatcc
mv ./flatcc/bin/flatcc /usr/bin/flatcc
rm -rf flatcc

# Setup git
git config --global core.editor "code --wait"