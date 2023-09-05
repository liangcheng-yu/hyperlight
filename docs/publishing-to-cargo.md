# Publishing Hyperlight Crates to Cargo

This document outlines how the various cargo feeds hosted in AzureDevOps were set up and how the project publishes to them

## Cargo Login

### One time setup

1. Ensure `reigstiry-auth` is enable in your `.cargo/config.toml`

    ```toml
    [unstable]
    registry-auth = true
    ```

1. Add the following your global cargo config file `$HOME/.cargo/config.toml`

    ```toml
    [registries]
    hyperlight_redist = { index = "sparse+https://pkgs.dev.azure.com/AzureContainerUpstream/hyperlight/_packaging/hyperlight_redist/Cargo/index/" }
    ```

### Login to AzureDevops cargo feeds

1. From the repo root run:

    ```bash
    just cargo-login
    ```

## Cargo Login for CI

Our CI jobs use PATs to login to the AzureDevOps cargo feeds.
Most CI jobs should use a PAT with the `Packaging (read)` scope (release jobs will need the `Packaging (read and write)` scope).

To login into internal cargo feeds using a PAT run `just cargo-login-ci` and ensure the `PAT` environment variable is set with a PAT with the appropriate scope.

Example using a read-only PAT:

```yaml
- name: Cargo login
  env:
    PAT: ${{ secrets.ADO_CARGO_RO_PAT }}
  run: just cargo-login-ci
```

### Rotating PATs

1. Generate a new PAT

    1. Navigate to https://dev.azure.com/AzureContainerUpstream/_usersSettings/tokens

    1. Click `+ New Token` button

    1. Set an appropriate time

    1. Selected either `Packaging (read)` or `Packaging (read and write)` scope as appropriate

    1. Click `Create`

    1. Take note of the PAT

1. Update the github Secrets

    1. Navigate to https://github.com/deislabs/hyperlight/settings/secrets/actions

    1. Click the `Edit` button for the appropriate secret - **ADO_CARGO_RO_PAT** for read-only access or **ADO_CARGO_RW_PAT** for read-write access

    1. Paste the new PAT into the `Value` field and click `Update secret` button.

## rust-vmm crates

Cargo requires that all dependant crates be published to a cargo feed in order to publish a crate.
Hyperlight depends on two crates (mshv-bindings and msh-ioctls) which are not currently published to a cargo feed so we publish them to a **redist** feed in AzureDevOps until the owners publish them elsewhere.

### Publishing rust-vmm crates to our **redist** feed

1. Clone the https://github.com/rust-vmm/mshv repository locally

1. Checkout the desired branch/tag/commit

    > Note: at the time of writing this, hyperlight pinned to rev `52edcf4`

1. Login to the AzureDevOps feeds (See [Cargo Login](#cargo-login) above)

1. Publish the **mshv-bindings** crate

    1. Run cargo publish

    ```bash
    cargo publish --registry hyperlight_redist --manifest-path mshv-bindings/Cargo.toml
    ```

1. Publish the **mshv-ioctls** crate

    1. Update `mshv-ioctls/Cargo.toml` by adding the a **version** and a **registry** to the mshv-bindings dependency

        ```toml
        [dependencies]
        ...
        mshv-bindings = {path = "../mshv-bindings", features = ["fam-wrappers"], version="*", registry="hyperlight_redist" }
        ...
        ```

    1. Commit the changes locally so cargo doesn't complain

    1. Publish the **mshv-ioctls** crate

        ```bash
        cargo publish --registry hyperlight_redist --manifest-path mshv-ioctls/Cargo.toml 
        ```
