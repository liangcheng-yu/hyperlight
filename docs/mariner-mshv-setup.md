# Set up a Mariner VM with MSHV in Azure

The easiest way to set up a Mariner VM with MSHV is to use an image provided by the Hyperlight team.

Access to the image is granted by joining the `Hyperlight-Cargo-Readers` [idweb](https://idweb.microsoft.com/identitymanagement/default.aspx) security group.
One you have access to the images search for the [hyperlight-dev image](https://ms.portal.azure.com/#@microsoft.onmicrosoft.com/resource/subscriptions/22c0772f-8056-4c9a-a7d7-d1b17fd277d4/resourceGroups/dev-images/providers/Microsoft.Compute/galleries/hyperlight/images/hyperlight-dev/overview) in the Hyperlight Azure Compute Gallery and create a VM from it.

This image is built using the following packer resources:

- [Packer file](../dev/packer/hyperlight.pkr.hcl)
- [Configuration script](../dev/packer/hyperlight.sh)

More information on how to create your own image from scratch can be found [here](https://www.osgwiki.com/wiki/LSG/Distro/Linux_in_Dom0/Nested#Obtaining_Dom0_images).

> Note - Currently the image is only available in the `westus2` and `westus3` regions. Please reach out to the Hyperlight team if you need the image in a different region.

An example terraform script to set up a VM can be found [here](../dev/mariner/main.tf).

## VM SKU Requirements

- Nested virtualization is required to run a VM with MSHV enabled. To check if your VM SKU supports nested virtualization, look up your desired VM Size / SKU Family (ex: [dv5](https://learn.microsoft.com/en-us/azure/virtual-machines/dv5-dsv5-series)) and search for "Nested Virtualization" on the SKU Family's page.

## VM SKU Recommendations

- We recommend starting off with either the [Dsv5](https://learn.microsoft.com/en-us/azure/virtual-machines/sizes/general-purpose/dsv5-series?tabs=sizebasic) (for Intel processors) or the [Dasv5](https://learn.microsoft.com/en-us/azure/virtual-machines/sizes/general-purpose/dasv5-series?tabs=sizebasic) (for AMD processors) series of VMs. These are the most cost-effective VMs that support nested virtualization.
- 2 vCPUs and 8 GB of memory is the minimum recommended configuration for a Mariner VM with MSHV enabled.
