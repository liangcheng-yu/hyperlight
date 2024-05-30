packer {
  required_plugins {
    azure = {
      source  = "github.com/hashicorp/azure"
      version = "~> 2"
    }
  }
}

variable location {
  type = string
  description = "Azure location (region) where the image will be produced and persisted"
  default = "westus3"
}

variable vm_size {
  type = string
  description = "Default VM Size upon which the image will be built."
  default = "Standard_D4as_v5"
}

variable image_name {
  type = string
  description = "The name of Azure image to use when pushing outputs to Azure. Default: hyperlight-ci"
  default = "hyperlight-ci"
}

locals {
  current_time = timestamp()
  major = formatdate("YYYY", local.current_time)
  minor = formatdate("MM", local.current_time)
  patch = formatdate("DDhhmmss", local.current_time)
  end_of_life = timeadd(local.current_time, "4536h") # 24hrs * 180 days
}

source "azure-arm" "hyperlight_dev" {
  azure_tags = {
    env = "development"
  }
  use_azure_cli_auth = true
  image_publisher                   = "MicrosoftCBLMariner"
  image_offer                       = "cbl-mariner"
  image_sku                         = "cbl-mariner-2-gen2"
  location                          = var.location
  managed_image_name                = var.image_name
  managed_image_resource_group_name = "dev-images"
  os_type                           = "Linux"
  vm_size                           = var.vm_size
  shared_image_gallery_destination {
    resource_group = "dev-images"
    gallery_name = "hyperlight"
    image_name = var.image_name
    image_version = "${local.major}.${local.minor}.${local.patch}"
    replication_regions = [var.location]
  }
  shared_gallery_image_version_end_of_life_date = local.end_of_life
}

build {
  sources = ["source.azure-arm.hyperlight_dev"]

  provisioner "shell" {
    execute_command = "echo 'packer' | sudo -S sh -c '{{ .Vars }} {{ .Path }}'"
    script = "${path.root}/scripts/provision.sh"
  }
}
