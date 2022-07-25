#!/bin/bash
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
    echo "Usage: $0 -g resource-group -l location -t github-runner-token -a tailscale-authkey"
    echo "The Tailscale Authkey should be pre auhtorised otherwise cloud-init will hang"
    exit 1
}

LOCATION=""
RESOURCE_GROUP=""
TOKEN=""
AUTHKEY=""

while getopts ":l:g:t:a:" opt; do
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
  - mkdir actions-runner 
  - cd actions-runner
  - curl -o actions-runner-linux-x64-2.294.0.tar.gz -L https://github.com/actions/runner/releases/download/v2.294.0/actions-runner-linux-x64-2.294.0.tar.gz
  - echo "a19a09f4eda5716e5d48ba86b6b78fc014880c5619b9dba4a059eaf65e131780  actions-runner-linux-x64-2.294.0.tar.gz" | shasum -a 256 -c
  - tar xzf ./actions-runner-linux-x64-2.294.0.tar.gz
  - chown azureuser:azureuser -R .
  - su azureuser -c "./config.sh --url https://github.com/deislabs --token ${TOKEN} --name hyperlight-runner-linux-kvm --labels linux,kvm"
  - ./svc.sh install azureuser
  - ./svc.sh start
EOF

az vm create --resource-group "${RESOURCE_GROUP}" --location "${LOCATION}" --size Standard_D2ds_v5 --name hyperlight-runner-linux-kvm --image Canonical:0001-com-ubuntu-server-focal:20_04-lts:latest --admin-username azureuser --generate-ssh-keys --public-ip-address "" --custom-data cloud-init.txt