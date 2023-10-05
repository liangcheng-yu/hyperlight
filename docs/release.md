# Releasing a new Hyperlight version to Cargo

This document details the process of releasing a new version of Hyperlight to the [Azure-internal Cargo feeds](https://dev.azure.com/AzureContainerUpstream/hyperlight/_artifacts/feed/hyperlight_packages_test). It's intended to be used as a checklist for the developer doing the release. The checklist is represented in the below sections.

## Create a tag

When the `dev` branch has reached a state in which you want to release a new Cargo version, you should create a tag. Although you can do this from the GitHub releases page, we currently recommend doing the tag from the command line (we will revisit this in the future). Do so with the following commands:

```shell
$ git tag -a v0.3.4 -m"A brief description of the release"
```

>Note: we'll use `v0.3.4` as the version for the above and all subsequent instructions. You should replace this with the version you're releasing. Make sure your version follows [SemVer](https://semver.org) conventions as closely as possible, and is prefixed with a `v` character

## Create a GitHub release

After you've created your tag, do the following:

1. Go to the GitHub releases page to [draft a new release](https://github.com/deislabs/hyperlight/releases/new)
2. Select the tag you just created (`v0.3.4` in the previous example)
3. Add a title for your release
4. Click the "Generate release notes" button to auto-generate a changelog
5. Check "Set as a pre-release" (for now; we will change this in the future)
6. Click the green "Publish release" button

## Change versions in `Cargo.toml` files

After you've created the release, some CI jobs will run. When they finish, you'll have a new branch in the repository called `release/v0.3.4`. The second-to-last step before we can publish a new version is to update the `Cargo.toml` files in the repository to reflect the new version. Do so by doing the following:

1. Cutting a new branch from `release/v0.3.4`:
    ```shell
    $ git checkout -b v0.3.4-versions origin/release/v0.3.4
    ```
2. Updating the following files to reflect the `0.3.4` version (note the lack of the `v` prefix!):
    - [`hyperlight_host/Cargo.toml`](/src/hyperlight_host/Cargo.toml)
    - [`hyperlight_testing/Cargo.toml`](/src/hyperlight_testing/Cargo.toml)
    - [`hyperlight_capi/Cargo.toml`](/src/hyperlight_capi/Cargo.toml)
3. Opening a new pull request (PR) to merge your `v0.3.4-versions` branch into the `release/v0.3.4` branch

## Run the publish job

After you merge the PR you created in the previous section, go to the ["Publish crates to intenral cargo registry"](https://github.com/deislabs/hyperlight/actions/workflows/CargoPublish.yml) Github actions workflow (yup, `intenral` is misspelled, and we like it that way!) and do the following:

1. Click the "Run workflow" button near the top right
2. Select the `release/0.3.4` branch in the resulting dropdown
3. Click the green "Run workflow" button

After step 3, the job will start and you'll have the following 3 crates published to the internal Azure DevOps Cargo feeds upon completion:

- `hyperlight_capi`
- `hyperlight_host`
- `hyperlight_testing`



