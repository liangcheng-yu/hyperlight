# Third Party Library Use

This project makes use of the following third party libraries, each of which is contained in a subdirectory of `third_party` with a LICENSE/README file in the root of the subdirectory.

## dlmalloc

This is a version (aka dlmalloc) of malloc/free/realloc written by
Doug Lea and dowloaded from [here](https://gee.cs.oswego.edu/pub/misc/malloc.c)

Changes have been applied to the original code for Hyperlight using this [patch](./dlmalloc/malloc.patch), you can download the original code and apply patches as follows:

```console
cd src/HyperlightGuest/third_party/dlmalloc
curl -Lv -o malloc.h https://gee.cs.oswego.edu/pub/misc/malloc.h
curl -Lv -o malloc.c https://gee.cs.oswego.edu/pub/misc/malloc.c
git apply --whitespace=nowarn --verbose malloc.patch
cd ../../../..
```
Or run `just update-dlmalloc` from the root of the repository to do this automatically.

## printf

This implementation of printf is from [here](https://github.com/mpaland/printf.git)
The copy was taken at version at [version 4.0](https://github.com/mpaland/printf/releases/tag/v4.0.0)

The code is included as a git sub tree[here](./printf/), you can initialise and update it as follows:

1. Add the repo as a remote

```console
git remote add -f printf https://github.com/mpaland/printf
```

1. Add the subtree

```console
git subtree add --prefix src/HyperlightGuest/third_party/printf printf v4.0.0 --squash
```

Changes have been applied to the original code for Hyperlight using this [patch](./printf/printf.patch)

1. Apply the patch

```console
git apply --whitespace=nowarn --verbose printf.patch
```

To update the subtree to a new version, run the following:

```console
git fetch printf
git subtree pull --prefix src/HyperlightGuest/third_party/printf printf VERSION --squash
```
