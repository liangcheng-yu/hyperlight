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

variable "prefix" {
  default = "hyperlight"
}

# Note: Choose vmsize that supports Nested Virtualization
variable "vmsize" {
  default = "Standard_D2_v4"
}

variable "location" {
  default = "southcentralus"
}

variable "sshkeypath" {
  default = "~/.ssh/id_rsa.pub"
}

variable "tailscale_auth_key" {}

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

  security_rule {
    name                       = "SSH"
    priority                   = 1001
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "22"
    source_address_prefix      = "*"
    destination_address_prefix = "*"
  }
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

resource "azurerm_virtual_machine" "main" {
  name                             = "${var.prefix}-vm"
  location                         = azurerm_resource_group.rg.location
  resource_group_name              = azurerm_resource_group.rg.name
  network_interface_ids            = [azurerm_network_interface.nic.id]
  vm_size                          = var.vmsize
  delete_os_disk_on_termination    = true
  delete_data_disks_on_termination = true
  storage_image_reference {
    publisher = "MicrosoftCBLMariner"
    offer     = "cbl-mariner"
    sku       = "cbl-mariner-2-kata"
    version   = "latest"
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
    custom_data    = templatefile("${path.module}/init.tpl", {
      tailscale_auth_key = var.tailscale_auth_key
    })
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

