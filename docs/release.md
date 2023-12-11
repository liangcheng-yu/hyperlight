# Releasing a new Hyperlight version to Cargo

This document details the process of releasing a new version of Hyperlight to the [Azure-internal Cargo feeds](https://dev.azure.com/AzureContainerUpstream/hyperlight/_artifacts/feed/hyperlight_packages_test). It's intended to be used as a checklist for the developer doing the release. The checklist is represented in the below sections.

## Update cargo.toml Versions - Optional

This step is optional as version numbers in Cargo.toml are updated in the [Release Pipeline](./../.github/workflows/CargoPublish.yml) but doing this manually means that the version number is updated in the dev branch to reflect what will become this latest release version.

For the crates hyperlight_host, hyperlight_capi and hyperlight_flatbuffers, update the version number in the Cargo.toml file to the new version number. In addition update any references to these crates in the Cargo.toml files of the other crates in the workspace.

Its not strictly necessary to update all the crates version numbers since some of them may not have chnaged, but since the release pipeline will update all the crates version numbers, you should follow the same process to ensure that the version numbers are consistent.

Create a PR with these changes and merge them into the dev branch.

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

## Run the publish job

After the release branch is created by automation, go to the ["Publish crates to intenral cargo registry"](https://github.com/deislabs/hyperlight/actions/workflows/CargoPublish.yml) Github actions workflow (yup, `intenral` is misspelled, and we like it that way!) and do the following:

1. Click the "Run workflow" button near the top right
1. Select the `release/v0.3.4` branch in the resulting dropdown
    - Optionally specific the package version. By default the workflow will publish crates with a version that matches the release branch the workflow is run against (ex: If run against `release/v0.3.4` then crates with versions `0.3.4` will be published). If you want to publish with a different version (in case we need to patch a release branch for example) then specify the version before clicking **Run Workflow**.
1. Click the green **Run workflow** button

After step 3, the job will start and you'll have the following 3 crates published to the [internal Azure DevOps Cargo feeds](https://dev.azure.com/AzureContainerUpstream/hyperlight/_artifacts/feed/hyperlight_packages_test) upon completion:

- `hyperlight_capi`
- `hyperlight_host`
- `hyperlight_testing`
