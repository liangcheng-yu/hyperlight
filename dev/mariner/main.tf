/*
Copyright 2024 The Hyperlight Authors.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/


# We strongly recommend using the required_providers block to set the
# Azure Provider source and version being used
terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~>3"
    }
  }
}

# Configure the Microsoft Azure Provider
provider "azurerm" {
  features {}
}

# Needed to access the shared image gallery images in our hyperlight-ci sub
provider "azurerm" {
  alias  = "alias"
  subscription_id = "22c0772f-8056-4c9a-a7d7-d1b17fd277d4"
  features {}
}


variable "prefix" {
  default = "hyperlight"
}

# Note: Choose vmsize that supports Nested Virtualization
variable "vmsize" {
  default = "Standard_D2s_v5"
}

variable "location" {
  default = "westus2"
}

variable "sshkeypath" {
  default = "~/.ssh/id_rsa.pub"
}

# Create a resource group
resource "azurerm_resource_group" "rg" {
  name     = "${var.prefix}-dev"
  location = var.location
}

resource "azurerm_virtual_network" "main" {
  name                = "${var.prefix}-network"
  address_space       = ["10.0.0.0/16"]
  location            = azurerm_resource_group.rg.location
  resource_group_name = azurerm_resource_group.rg.name
}

resource "azurerm_subnet" "internal" {
  name                 = "internal"
  resource_group_name  = azurerm_resource_group.rg.name
  virtual_network_name = azurerm_virtual_network.main.name
  address_prefixes     = ["10.0.2.0/24"]
}

# Create public IPs
resource "azurerm_public_ip" "public_ip" {
  name                = "${var.prefix}PublicIP"
  location            = azurerm_resource_group.rg.location
  resource_group_name = azurerm_resource_group.rg.name
  allocation_method   = "Dynamic"
}

# Create Network Security Group and rule
resource "azurerm_network_security_group" "nsg" {
  name                = "${var.prefix}NetworkSecurityGroup"
  location            = azurerm_resource_group.rg.location
  resource_group_name = azurerm_resource_group.rg.name
}

resource "azurerm_network_interface" "nic" {
  name                = "${var.prefix}-nic"
  location            = azurerm_resource_group.rg.location
  resource_group_name = azurerm_resource_group.rg.name

  ip_configuration {
    name                          = "${var.prefix}configuration1"
    subnet_id                     = azurerm_subnet.internal.id
    private_ip_address_allocation = "Dynamic"
    public_ip_address_id          = azurerm_public_ip.public_ip.id
  }
}

resource "azurerm_network_interface_security_group_association" "ssh_nsg" {
  network_interface_id      = azurerm_network_interface.nic.id
  network_security_group_id = azurerm_network_security_group.nsg.id
}

data "azurerm_shared_image" "image" {
  name                = "hyperlight-dev"
  gallery_name        = "hyperlight"
  resource_group_name = "dev-images"

  provider = azurerm.alias
}

resource "azurerm_virtual_machine" "main" {
  name                             = "${var.prefix}-vm"
  location                         = azurerm_resource_group.rg.location
  resource_group_name              = azurerm_resource_group.rg.name
  network_interface_ids            = [azurerm_network_interface.nic.id]
  vm_size                          = var.vmsize
  delete_os_disk_on_termination    = true
  delete_data_disks_on_termination = true
  storage_image_reference {
    id = "${data.azurerm_shared_image.image.id}"
  }
  storage_os_disk {
    name              = "myosdisk1"
    caching           = "ReadWrite"
    create_option     = "FromImage"
    managed_disk_type = "Standard_LRS"
    disk_size_gb      = "40"
  }
  os_profile {
    computer_name  = "hyperlightdev"
    admin_username = "azureuser"
  }
  os_profile_linux_config {
    disable_password_authentication = true
    ssh_keys {
      key_data = file(var.sshkeypath)
      path     = "/home/azureuser/.ssh/authorized_keys"
    }
  }
  tags = {
    environment = "${var.prefix}-dev"
  }
}

data "azurerm_public_ip" "dynamic" {
  name                = "${var.prefix}PublicIP"
  resource_group_name = azurerm_resource_group.rg.name
  depends_on          = [azurerm_public_ip.public_ip, azurerm_virtual_machine.main]
}

output "pub_ip" {
  value = data.azurerm_public_ip.dynamic.ip_address
}

