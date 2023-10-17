# Releasing a new Hyperlight version to Cargo

This document details the process of releasing a new version of Hyperlight to the [Azure-internal Cargo feeds](https://dev.azure.com/AzureContainerUpstream/hyperlight/_artifacts/feed/hyperlight_packages_test). It's intended to be used as a checklist for the developer doing the release. The checklist is represented in the below sections.

## Create a tag

When the `dev` branch has reached a state in which you want to release a new Cargo version, you should create a tag. Although you can do this from the GitHub releases page, we currently recommend doing the tag from the command line (we will revisit this in the future). Do so with the following commands:

```bash
$ git tag -a v0.3.4 -m"A brief description of the release"
$ git push origin v0.3.4 # if you've named your git remote for the deislabs/hyperlight repo differently, change 'origin' to your remote name
```

>Note: we'll use `v0.3.4` as the version for the above and all subsequent instructions. You should replace this with the version you're releasing. Make sure your version follows [SemVer](https://semver.org) conventions as closely as possible, and is prefixed with a `v` character

## Create a release branch (no manual steps)

After you push your new tag in the previous section, the ["Create a Release Branch"](https://github.com/deislabs/hyperlight/actions/workflows/CreateReleaseBranch.yml) CI job will automatically run. When this job completes, a new `release/v0.3.4` branch will be automatically created for you.

## Create a new GitHub release (no manual steps)

After the previous CI job runs to create the new release branch, the ["Create a Release"](https://github.com/deislabs/hyperlight/actions/workflows/CreateRelease.yml) job will see the new branch and automatically run. When this job is done, a new [GitHub release](https://github.com/deislabs/hyperlight/releases) will be created for you. This release is not strictly necessary for releasing a new cargo crate to the internal feed, but it is necessary to house other artifacts (e.g. `simpleguest.exe`, `callbackguest.exe`, etc)

## Update versions in the `Cargo.toml` files

After the release branch was created (in the "Create a release branch" section above), you have to open a PR against that branch to reflect the new version. Do so by doing the following:

1. Cutting a new branch from `release/v0.3.4`:
    ```shell
    $ git checkout -b v0.3.4-cargo-toml origin/release/v0.3.4
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



