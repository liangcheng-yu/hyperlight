# AzureDevOps Cargo feeds - Development Notes

This document outlines how the various cargo feeds hosted in AzureDevOps were set up and how the project publishes to them.

## Feed Locations

Currently, we use the following feed(s) set up in AzureDevOps:

- [hyperlight_packages](https://dev.azure.com/AzureContainerUpstream/hyperlight/_artifacts/feed/hyperlight_packages): This feed is used to distribute crates produced by the Hyperlight project.

## Feed Access

To gain access please navigate to [IDWeb](http://aka.ms/idweb) and join the **Hyperlight-Cargo-Readers**  security group.

## Cargo Login

To log in the AzureDevOps cargo from any of the Hyperlight repositories run:

```bash
az login
just cargo-login
```

## Authentication in GitHub workflows

All the GitHub workflows in the various Hyperlight repositories use OIDC tokens to authenticate with AzureDevOps using Azure Entra ID identities.
This section outlines how the various components are configured.

### App Registrations

The GitHub workflow use two different Application Registrations. One is used in pull request workflows and only allows read access to the feeds. The other is used in publishing workflows and has read/write access to the feeds.

- [AzureContainerUpstream-Hyperlight-AzureDevops-CargoFeed-ReadAccess](https://ms.portal.azure.com/#view/Microsoft_AAD_RegisteredApps/ApplicationMenuBlade/~/Overview/appId/2f9dcd38-2853-4335-9c79-e7163845abbf/isMSAApp~/false)- ID: 2f9dcd38-2853-4335-9c79-e7163845abbf
- [AzureContainerUpstream-Hyperlight-AzureDevops-CargoFeed-ReadWriteAccess](https://ms.portal.azure.com/#view/Microsoft_AAD_RegisteredApps/ApplicationMenuBlade/~/Overview/appId/01ce0d32-4a0b-4a97-9262-9ae410e4d3f3/isMSAApp~/false)- ID: 01ce0d32-4a0b-4a97-9262-9ae410e4d3f3

### Azure Login / Federated Credentials

The GitHub workflows use the `Azure Login` action to authenticate with Azure. Each application registration has a set of federated credentials that dictate which GitHub workflows can use the application registration.

The projects in Hyperlight use **PullRequest** and **Environment** scopes.

- `PullRequest` scopes (ex: repo:deislabs/hyperlight:pull_request) allow any pull request to use the application registration.
- `Environment` scopes (ex: repo:deislabs/hyperlight:environment:release) allow any workflow that uses the `Environment` scope to use the application registration.
  - Environment scopes are used because currently Azure does not support wildcards in `Branch` scopes. To avoid needing to manually add a new scope for each release branch we instead run the release workflows in a **release** environment. See the [GitHub Environments](#github-environments) section for more details.

To edit the scopes for federated authentication for a given application registration:

- Navigate to the application registration in the azure portal
- Click on `Manage` -> `Certificates & secrets` on the side menu
- Click on `Federated credentials` in the table on the page

### GitHub Secrets

The GitHub workflows use the following secrets to authenticate with Azure:

| Secret Name                             | Description                                                                                                              |
|-----------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
| ADO_HYPERLIGHT_CARGO_RO_AZURE_CLIENT_ID | The client ID for the `AzureContainerUpstream-Hyperlight-AzureDevops-CargoFeed-ReadAccess` application registration      |
| ADO_HYPERLIGHT_CARGO_RW_AZURE_CLIENT_ID | The client ID for the `AzureContainerUpstream-Hyperlight-AzureDevops-CargoFeed-ReadWriteAccess` application registration |
| AZURE_TENANT_ID                         | The tenant ID for the Azure AD tenant                                                                                    |

These secrets are set at the organization level in the **deislabs** GitHub organization.

### GitHub Environments

We are using GitHub Environments because Azure does not support wildcards in `Branch` scopes. This allows us to run the release workflows in a **release** environment and use the `Environment` scope for federated authentication.

Each Hyperlight repository has an environment named `release` and the use of this environment is restricted to the `dev` and `release/**` branches (we release a 'latest' package from `dev` branch).

Note: Many of the workflows in the Hyperlight repositories are called from both PR jobs and release jobs.
We support this by taking an environment name as an optional input and set this when the workflows are called from CreateRelease.yml (or similar) workflows and GitHub seems OK setting a blank environment for jobs in workflows.

## New Repo onboarding

When a new repository is created in the Hyperlight organization the following steps must be taken to enable reading from and publishing to the Hyperlight feeds:

- Add new federated scopes for the new repository to the `AzureContainerUpstream-Hyperlight-AzureDevops-CargoFeed-ReadAccess` and `AzureContainerUpstream-Hyperlight-AzureDevops-CargoFeed-ReadWriteAccess` application registrations.
  - See [Azure Login / Federated Credentials](#azure-login--federated-credentials) for more details.
- Add the new repository to the list of repositories that use the **deislabs** org with GitHub secrets
  - See [GitHub Secrets](#github-secrets) for more details.
