# Set up a Mariner VM with mshv for local use

This document describes how to set up a Mariner VM with MSHV.

## Prerequisites

There are two ways to get a vhd with Mariner and mshv before creating your virtual machine:

* [Download the latest Mariner release](#download-latest-mariner-release)
* Use AzCopy to [copy a vhd from another developer](#copy-another-developers-mariner-vhd) (recommended)

### Download latest Mariner release

Follow the instructions [here](https://www.osgwiki.com/wiki/LSG/Distro/Linux_in_Dom0/Nested#Obtaining_Dom0_images) to obtain a Mariner image with mshv and set up in HyperV. If you don't have access contact **@arschles** or **@simongdavies**.

Find a run where both the stages have succeeded, select the run, under the summary select the "Artifacts" summary, select `dom0 out->images->dom0` and download either the `vhdx` or `vhdx.xz` file. If you download the `.xz` file, you will need to extract it with the below command before using it:

```shell
xz -d -v <filename>
```

### Copy another developer's Mariner vhd

1. Ask another developer on the project for a link to their Mariner vhd in their Azure Blob Storage account, including an [unexpired SAS token][blob-sas-token] so that you can authenticate. The link will look like this: 
   ```
   https://STORAGEACCOUNT.blob.core.windows.net/CONTAINER/dom0-mariner-dev-latest-x86_64.vhd?SAS_TOKEN_QUERY_STRING_PARAMETERS
   ```
1. Download [azcopy] and add the binary to your PATH
   ```
   curl -o azcopy.tar.gz https://aka.ms/downloadazcopy-v10-linux
   tar -zxf azcopy.tar.gz
   # adjust the directory name below to match the version that you downloaded
   sudo mv azcopy_linux_amd64_10.17.0/azcopy /usr/local/bin/
   ```
1. If you will use the vhd on Azure:
   1. Create an Azure Blob Storage account (or reuse an existing one) and create a container to transfer the vhd into.
   1. Generate a SAS token link for your container, inserting the name of the blob to the URL before the query string. It should look like the template URL shown at the the top of this section.
   1. Transfer the vhd into your storage account. The transfer happens within Azure, so it is fine to run this command from your local machine, and it won't cause you to download and then upload the (large) vhd.
      ```
      azcopy copy SOURCE_BLOB_URL_WITH_SAS_TOKEN DEST_BLOB_URL_WITH_SAS_TOKEN
      ```
1. Otherwise if you are running the VM locally, download it with:
   ```
   azcopy copy SOURCE_BLOB_URL_WITH_SAS_TOKEN DESTINATION_FILE_PATH
   ```

[azcopy]: https://learn.microsoft.com/en-us/azure/storage/common/storage-use-azcopy-v10#download-azcopy
[blob-sas-token]: https://learn.microsoft.com/en-us/azure/cognitive-services/translator/document-translation/how-to-guides/create-sas-tokens?tabs=Containers

## Set up the VM

Before starting the VM, ensure the disk is 20GB or larger in size. Some VHDX disk images are too small for all the Hyperlight prequisites. See [here](https://docs.microsoft.com/en-us/virtualization/hyper-v-on-windows/user-guide/expand-virtual-hard-disk) for instructions on how to expand the VHDX. Many of the latest (as of this writing) Mariner disk images are of sufficient size, so this step will be unneccesary in that case.

Start the VM and login with the default credentials from the wiki above. As of this writing, they will be:

```shell
user: cloud
pass: Cloud123
```

## If you're launching on Azure

There are several steps necessary to set up a Mariner VM on Azure. There is a script described [here](https://www.osgwiki.com/wiki/LSG/Distro/Linux_in_Dom0/Nested#Setting_up_an_Azure_Dom0_VM) that you should use to do almost all of them.

To get the script, you'll need access to the https://microsoft.visualstudio.com/DefaultCollection/LSG/_git/lsg-tools repository. If you need access, contact **@arschles** or **@simongdavies**.

If you haven't already done so, ensure that the VHD image you got in the previous section is uploaded to an Azure Blob Store container in your Azure subscription. It's recommended to do the upload from Azure Cloud Shell or another VM in Azure because network speeds will be faster. The blob and container can be set to private access, but be sure to copy the URL to the VHD and set it to the environment variable `BLOB_STORE_IMAGE_LOCATION`, as it will be used in the subsequent step:

```shell
export BLOB_STORE_IMAGE_LOCATION=<full URL>
```

You also need to export your Azure Subscription ID:

```shell
export AZURE_SUBSCRIPTION_ID="YOUR AZURE SUB ID"
```

Finally, the command to launch the VM is slightly different than what's listed on the wiki:

```shell
./scripts/create_vm.sh -t linux-dom0 -n ${NAME_OF_NEW_VM} -s ${BLOB_STORE_IMAGE_LOCATION} --vm-nsg-rule SSH --subscription ${AZURE_SUBSCRIPTION_ID}
```

There are additional optional flags that you can specify, for example the resource group to create/reuse, you can find those flags in the source of the script.

## VM Configuration Steps

Finally, you'll need to log into your VM. If you launched it on Azure with the steps in the previous section, you'll need to use Azure Bastion for this. Once you're logged in, follow these steps:

1. Add a new user.
1. Optionally set the hostname for the VM by editing /etc/hostname and then restarting. The host name will be used later when registering the host with tailscale.
1. Install dnf `sudo yum install -y dnf`
1. Install Tailscale

   ```shell
   sudo dnf install -y 'dnf-command(config-manager)'
   sudo dnf config-manager --add-repo https://pkgs.tailscale.com/stable/fedora/tailscale.repo
   sudo dnf install -y tailscale
   sudo systemctl enable --now tailscaled
   sudo tailscale up --ssh
   ```

1. Authenticate with Tailscale
1. Authorize the new machine in Tailscale admin portal.

   If you're logged in via Azure Bastion, you can log out, SSH into the new machine using the Tailscale IP address, and continue the steps with the new SSH session. Delete the bastion if you aren't using it for other VMs.

1. Install vim `sudo dnf install -y vim`
1. Give the new user sudoer access. Edit `edit /etc/sudoers.d/90-dom0-users`
1. Add the new user to the mshv group `sudo usermod -G mshv -a $(whoami)`
1. Disable cloud user `sudo usermod -L cloud`

1. Install tools needed for development

   ```shell
   sudo dnf install -y \
   git \
   clang \
   lldb \
   dotnet-sdk-6.0 \
   binutils \
   valgrind \
   glibc-devel \
   kernel-headers \
   nano
   ```

1. Install rust `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
1. Install just `cargo install just`

## Dev/test Setup - not needed for GitHub Custom Runner

1. Configure your git and gh cli authentication. If you have ssh keys configured with github an easy way to do that is to forward you ssh keys with `ssh -A` when connecting to your VM and then running `gh auth setup-git` so that the gh cli uses them.
1. Clone Hyperlight `git clone git@github.com:deislabs/hyperlight.git`
1. Change to the Hyperlight directory and run `git submodule update --init`. Hyperlight uses submodules to pull in some dependencies such as munit. If you see munit errors when running tests, make sure you have the submodules cloned.
1. Install direnv `curl -sfL https://direnv.net/install.sh | bash`
1. Edit `~/.bashrc` and add `eval "$(direnv hook bash)"` to the end of the file.
1. Add env var to .envrc `echo "export HYPERV_SHOULD_BE_PRESENT=true" > ./hyperlight/.envrc`
1. Change to the Hyperlight directory and run `direnv allow`
1. Install GitHub CLI
   ```shell
   sudo dnf install 'dnf-command(config-manager)'
   sudo dnf config-manager --add-repo https://cli.github.com/packages/rpm/gh-cli.repo
   sudo dnf install gh
   ```
1. Download the test programs
  
   ```shell
   # set this to the release tag you want to download the test guests from
   # See https://github.com/deislabs/hyperlight/releases
   RELEASE_TAG="5c4ada5"
   mkdir -p src/tests/Guests/dummyguest/x64/debug/ && cd  src/tests/Guests/dummyguest/x64/debug/ && gh release download  ${RELEASE_TAG} -p 'dummyguest.exe' && cd -
   mkdir -p src/tests/Guests/dummyguest/x64/release/ && cd  src/tests/Guests/dummyguest/x64/release/ && gh release download  ${RELEASE_TAG} -p 'dummyguest.exe' && cd -
   mkdir -p src/tests/Guests/simpleguest/x64/debug/ && cd  src/tests/Guests/simpleguest/x64/debug/ && gh release download  ${RELEASE_TAG} -p 'simpleguest.exe' && cd -
   mkdir -p src/tests/Guests/simpleguest/x64/release/ && cd  src/tests/Guests/simpleguest/x64/release/ && gh release download  ${RELEASE_TAG} -p 'simpleguest.exe' && cd -
   mkdir -p src/tests/Guests/callbackguest/x64/debug/ && cd  src/tests/Guests/callbackguest/x64/debug/ && gh release download  ${RELEASE_TAG} -p 'callbackguest.exe' && cd -
   mkdir -p src/tests/Guests/callbackguest/x64/release/ && cd  src/tests/Guests/callbackguest/x64/release/ && gh release download  ${RELEASE_TAG} -p 'callbackguest.exe' && cd -
   ```

Now follow the instructions in the [Hyperlight README instructions for WSL2 or Linux](../README.md#wsl2-or-linux).

## Configuring a GitHub Actions self-hosted runner

- Go to [the self-hosted runner create page](https://github.com/organizations/deislabs/settings/actions/runners/new) and click the "Linux" radio button.
- Follow all steps up to but not including the `./run.sh` command
- Go to the [configure the runner as a service](https://docs.github.com/en/actions/hosting-your-own-runners/configuring-the-self-hosted-runner-application-as-a-service) documentation page and follow steps through the `sudo ./svc.sh status` command.
  - Make sure you're on the "Linux" tab

The total list of commands should look similar to:

```shell
mkdir actions-runner && cd actions-runner
curl -o actions-runner-linux-x64-2.298.2.tar.gz -L https://github.com/actions/runner/releases/download/v2.298.2/actions-runner-linux-x64-2.298.2.tar.gz
# optional: validate the hash
echo "SOME HASH  actions-runner-linux-x64-2.298.2.tar.gz" | shasum -a 256 -c
tar xzf ./actions-runner-linux-x64-2.298.2.tar.gz
./config.sh --url https://github.com/deislabs --token AAARJUJJFCJTBIMGC6PECA3DNQYRA
# Reminder: do not execute ./run.sh here
sudo ./svc.sh install
sudo ./svc.sh start
sudo ./svc.sh status
```

There is also a `.github/setup_runners.sh` script that automates the above steps.
