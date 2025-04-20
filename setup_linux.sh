######## Install KVM on Linux ########

# Check if hardware virtualization is supported.
# If the result is 0, enable Intel VT-x or AMD-V in BIOS.
egrep -c '(vmx|svm)' /proc/cpuinfo
# Check if the (x86) CPU supports 64-bit (should return >= 1).
egrep -c ' lm ' /proc/cpuinfo
# Check if the running kernel is 64-bit (should return 'x86_64' or 'amd64').
uname -m

sudo apt update && sudo apt upgrade -y
sudo apt-get install -y qemu-kvm libvirt-daemon-system libvirt-clients bridge-utils

sudo adduser `id -un` libvirt
sudo adduser `id -un` kvm
# Relogin.

# Verify.
sudo systemctl status libvirtd
lsmod | grep kvm
groups
virsh list --all

######## Install hyperlight dependencies ########
sudo apt install build-essential
# Install Rust.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
# Require both targets even if just targeting Linux.
cd hyperlight
rustup target add x86_64-unknown-none
rustup target add x86_64-pc-windows-msvc
cargo install just
# clang and llvm.
wget https://apt.llvm.org/llvm.sh
chmod +x ./llvm.sh
sudo ./llvm.sh 17 all
sudo ln -s /usr/lib/llvm-17/bin/clang-cl /usr/bin/clang-cl
sudo ln -s /usr/lib/llvm-17/bin/llvm-lib /usr/bin/llvm-lib
sudo ln -s /usr/lib/llvm-17/bin/lld-link /usr/bin/lld-link
sudo ln -s /usr/lib/llvm-17/bin/llvm-ml /usr/bin/llvm-ml
sudo ln -s /usr/lib/llvm-17/bin/ld.lld /usr/bin/ld.lld
sudo ln -s /usr/lib/llvm-17/bin/clang /usr/bin/clang

######## Build hyperlight ########
just build
just rg
cargo run --example hello-world
