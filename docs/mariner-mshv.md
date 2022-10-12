# Set up a Mariner VM with mshv for local use

This document describes how to set up a Mariner VM with MSHV.

## Prerequisites

Follow the instructions [here](https://www.osgwiki.com/wiki/LSG/Distro/Linux_in_Dom0/Nested#Obtaining_Dom0_images) to obtain a Mariner image with mshv and set up in HyperV. 

Find a run where both the stages have succeeded, select the run, under the summary select the "Artifacts" summary, select don0 out->images->dom0 and download either the `vhdx` or `vhdx.xz` file, if you download the `.xz` file, you will need to extract it with ` xz -d -v <filename>` before using it.

If you dont have access contact @simongdavies. 

# Set up the VM.

Before starting the VM expand the disk to 20GB, the default VHDX is too small for all the Hyperlight prequisites. See [here](https://docs.microsoft.com/en-us/virtualization/hyper-v-on-windows/user-guide/expand-virtual-hard-disk) for instructions on how to expand the VHDX.

Start the VM and login with the default credentials from the wiki above.

1. Add a new user.
1. Install dnf `sudo yum install dnf`
1. Install vim `sudo dnf install vim`
1. Give the new user sudoer access. Edit `edit /etc/sudoers.d/90-dom0-users`
1. Add the new user to the mshv group `sudo usermod -G mshv -a <username>`
1. Disable cloud user `sudo usermod -L cloud`
1. Install Tailscale
    ```
    dnf install 'dnf-command(config-manager)'
    sudo dnf config-manager --add-repo https://pkgs.tailscale.com/stable/fedora/tailscale.repo
    sudo dnf install tailscale
    sudo systemctl enable --now tailscale
    sudo tailscale up --ssh
    ```
1. Authenticate with Tailscale
1. Authorize the new machine in Tailscale admin portal.
1. ssh to new machine using tailscale IP address
1. Install git `sudo dnf install git`
1. Install clang `sudo dnf install clang`
1. Install dotnet 6 sdk `sudo dnf install dotnet-sdk-6.0`
1. Install rust `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
1. Install binutils `sudo dnf install binutils`
1. Install valgrind `sudo dnf install valgrind`
1. Install glibc-devel `sudo dnf install glibc-devel`
1. Install kernel-headers `sudo dnf install kernel-headers`
1. Install just `cargo install just`
1. Clone Hyperlight `git clone git@github.com:deislabs/hyperlight.git`
1. Install direnv `curl -sfL https://direnv.net/install.sh | bash`
1. Edit `~/.bashrc` and add `eval "$(direnv hook bash)"` to the end of the file.
1. Add env var to .envrc `echo "export HYPERV_SHOULD_BE_PRESENT=true" > ./hyperlight/.envrc`
1. cd to the Hyperlight directory and run `direnv allow`

Now follow the instructions in the [Hyperlight README](../README.md).
