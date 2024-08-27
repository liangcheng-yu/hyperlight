# Publishing mshv crates

This document outlines how to publish / update the MSHV crates in the `hyperlight_redist` feed in AzureDevOps.

## rust-vmm crates

Cargo requires that all dependant crates be published to a cargo feed in order to publish a crate.

Hyperlight depends on two crates (mshv-bindings and msh-ioctls) which are not currently published to a cargo feed, so we publish them to a **redist** feed in AzureDevOps until the owners publish them elsewhere.

## cargo login

To publish to the **redist** feed, you need to:

```sh
az login
just cargo-login # from the root of the Hyperlight repository
```

### Publishing rust-vmm crates to our **redist** feed

1. Clone the https://github.com/rust-vmm/mshv repository locally
2. Checkout the desired branch/tag/commit
3. Login to the AzureDevOps feeds (See [Cargo Login](#cargo-login) above)
4. Set crates to the desired version (See [Versioning](#version-number-format) below)
5. Publish the **mshv-bindings** crate:
    ```bash
    cargo publish --registry hyperlight_redist --manifest-path mshv-bindings/Cargo.toml
    ```
6. Publish the **mshv-ioctls** crate
    1. Update `mshv-ioctls/Cargo.toml` by adding a **version** and a **registry** to the mshv-bindings dependency
        ```toml
        [dependencies]
        ...
        mshv-bindings = {path = "../mshv-bindings", features = ["fam-wrappers"], version="<target version>", registry="hyperlight_redist" }
        ...
        ```
   2. Commit the changes locally so cargo doesn't complain
   3. Publish the **mshv-ioctls** crate
        ```bash
         cargo publish --registry hyperlight_redist --manifest-path mshv-ioctls/Cargo.toml 
         ```

### Version Number Format

Currently, we use the following format for version numbers for the rust-vmm crates: 0.1.2-YYYMMDD where YYYMMDD is the date the crates were published.

We have experimented with other formats such as including the commit hash in the version but found that `cargo ws version` only treats versions with a single `-` as a valid version (even when looking at dependencies).
