#!/bin/bash

# The value for tailscale authkey should be obtained by creating a new Auth key at https://login.tailscale.com/admin/settings/keys
# The value for github-runner-token should be obtained by creating a new runner at https://github.com/organizations/deislabs/settings/actions/runners/new 

set -euo pipefail

pushd -n $(pwd)

function cleanup()
{
    rm -f cloud-init.txt 
    popd
}

trap cleanup EXIT

cd /tmp

usage()
{
    echo "Creates custom GH action runners on Azure for Hyperlight"
    echo "Log into azure before runninig this script"
    echo "Usage: $0 -g resource-group -l location -t github-runner-token -a tailscale-authkey -n machine/runner name"
    echo "The Tailscale Authkey should be pre auhtorised otherwise cloud-init will hang"
    exit 1
}

LOCATION=""
RESOURCE_GROUP=""
TOKEN=""
AUTHKEY=""
MACHINE_NAME=""

while getopts ":l:g:t:a:n:" opt; do
    case "${opt}" in
        l)
            LOCATION=${OPTARG}
            ;;
        g)
            RESOURCE_GROUP=${OPTARG}
            ;;
        t)  
            TOKEN=${OPTARG}
            ;;
        a)  
            AUTHKEY=${OPTARG}
            ;;
        n)  
            MACHINE_NAME=${OPTARG}
            ;;
        *)
            usage
            ;;
    esac
done

# Create a resource group for the runners.
az group create --name "${RESOURCE_GROUP}" --location "${LOCATION}"

tee cloud-init.txt <<EOF
#cloud-config
package_upgrade: true
apt:
  sources:
    tailscale.list:
      source: deb https://pkgs.tailscale.com/stable/ubuntu focal main
      keyid: 2596A99EAAB33821893C0A79458CA832957F5868
packages:
  - tailscale
  - cpu-checker
  - qemu-kvm
runcmd:
  - tailscale up -authkey ${AUTHKEY} --ssh
  - echo 'net.ipv4.ip_forward = 1' | sudo tee -a /etc/sysctl.conf
  - echo 'net.ipv6.conf.all.forwarding = 1' | sudo tee -a /etc/sysctl.conf
  - sysctl -p /etc/sysctl.conf
  - adduser azureuser kvm
  - mkdir -p /usr/share/dotnet
  - chown azureuser:azureuser /usr/share/dotnet
  - mkdir actions-runner 
  - cd actions-runner
  - curl -o actions-runner-linux-x64-2.301.1.tar.gz -L https://github.com/actions/runner/releases/download/v2.301.1/actions-runner-linux-x64-2.301.1.tar.gz
  - sudo apt-get install -y build-essential clang valgrind
  - echo "3ee9c3b83de642f919912e0594ee2601835518827da785d034c1163f8efdf907  actions-runner-linux-x64-2.301.1.tar.gz" | shasum -a 256 -c
  - tar xzf ./actions-runner-linux-x64-2.301.1.tar.gz
  - chown azureuser:azureuser -R .
  - su azureuser -c "./config.sh --url https://github.com/deislabs --token ${TOKEN} --name ${MACHINE_NAME} --labels linux, kvm, kvm2"
  - ./svc.sh install azureuser
  - ./svc.sh start
EOF

az vm create --resource-group "${RESOURCE_GROUP}" --location "${LOCATION}" --size Standard_D2ds_v5 --name ${MACHINE_NAME} --image Canonical:0001-com-ubuntu-server-focal:20_04-lts:latest --admin-username azureuser --generate-ssh-keys --public-ip-address "" --custom-data cloud-init.txt
