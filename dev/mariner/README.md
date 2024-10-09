# Creating a Mariner / MSHV VM in Azure

This will walk you through creating a Mariner Linux Hyperlight development machine using Terraform using a image from the provided Hyperlight Azure Compute / Shared Image Gallery.

## Tools to install

- [Terraform](https://developer.hashicorp.com/terraform/tutorials/aws-get-started/install-cli)
- [Azure CLI](https://learn.microsoft.com/en-us/cli/azure/install-azure-cli)

## Create the Machine

```shell
terraform init
terraform plan -out main.tfplan
terraform apply main.tfplan
```

## Login to the Machine

> See note below about the ssh key path

```shell
ssh azureuser@hyperlightdev -A
```

## Destroy Your Mariner Development Machine

```shell
terraform destroy
```

or

```shell
az group delete -n hyperlight-dev -y --no-wait
```

## Defaulted Variables

You can specify several variables to the Terraform plan.

- prefix: the prefix applied to the resource group name in Azure (default: hyperlight)
- vmsize: the VM size used for the machine (default: Standard_D2_v4)
- location: the Azure region which the machine is deployed (default: southcentralus)
- sshkeypath: the path to the ssh public key added to the authorized_keys file on the machine (default: ~/.ssh/id_rsa.pub)
