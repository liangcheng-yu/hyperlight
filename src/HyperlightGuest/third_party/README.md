# Third Party Library Use

This project makes use of the following third party libraries, each of which is contained in a subdirectory of `third_party` with a COPYRIGHT/LICENSE/README file in the root of the subdirectory.

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
Changes have been applied to the original code for Hyperlight using this [patch](./printf/printf.patch)

The code is included as a git sub tree [here](./printf/), you can initialise and update it as follows:

1. Add the repo as a remote

    ```console
    git remote add -f printf https://github.com/mpaland/printf
    ```

1. Add the subtree

    ```console
    git subtree add --prefix src/HyperlightGuest/third_party/printf printf v4.0.0 --squash
    ```

1. Apply the patch

    ```console
    git apply --whitespace=nowarn --verbose printf.patch
    ```

To update the subtree to a new version, run the following:

```console
git fetch printf
git subtree pull --prefix src/HyperlightGuest/third_party/printf printf VERSION --squash
```

## libc

A partial version of musl libc is used by Hyperlight and is located in the [musl](./libc/musl) directory.

The current version is release [v1.2.3](https://git.musl-libc.org/cgit/musl/tag/?h=v1.2.3). Many files have been deleted and changes have been made to some of the remaining files, those changes can be applied using [this](./libc/musl-libc.patch) patch.

The code is included as a git sub tree[here](./libc/musl), you can initialise and update it as follows:

1. Add the musl libc repo as a remote

    ```console
    git remote add -f musllibc git://git.musl-libc.org/musl
    ```

1. Add the subtree

    ```console
    git subtree add --prefix src/HyperlightGuest/third_party/libc/musl musllibc v1.2.3 --squash
    ```

1. Apply the patch

    ```console
    git apply --whitespace=nowarn --verbose src/HyperlightGuest/third_party/libcmusl-libc.patch
    ```

Note: The alltypes.h file was generated from the alltypes.h.in files, you can generate before applying the patch as follows:

```console
cd src/HyperlightGuest/third_party/libc/musl/
sed -f ./tools/mkalltypes.sed ./arch/x86_64/bits/alltypes.h.in ./include/alltypes.h.in > ./arch/x86_64/bits/alltypes.h
```

To update the subtree to a new version, run the following:

```console
git fetch musllibc
git subtree pull --prefix src/HyperlightGuest/third_party/libc/musl musllibc VERSION --squash
```

## flatcc

flatcc is used for both C code generation from flatbuffers schemas and runtime reading and building of buffers. The current version is [v0.6.1](https://github.com/dvidelabs/flatcc/releases/tag/v0.6.1).

The code is included as a git sub tree[here](./flatcc), you can initialise and update it as follows:

```console
 git remote add -f flatcc https://github.com/dvidelabs/flatcc.git
 ```

 ```console
  git subtree add --prefix src/HyperlightGuest/third_party/flatcc flatcc v0.6.1 --squash
 ```

 We only need a few files from flatcc for compilaton so most of the files can be deleted

 ```console
rm -r .\src\HyperlightGuest\third_party\flatcc\config 
rm -r .\src\HyperlightGuest\third_party\flatcc\doc   
rm -r .\src\HyperlightGuest\third_party\flatcc\external
rm -r .\src\HyperlightGuest\third_party\flatcc\reflection
rm -r .\src\HyperlightGuest\third_party\flatcc\samples
rm -r .\src\HyperlightGuest\third_party\flatcc\scripts
rm -r .\src\HyperlightGuest\third_party\flatcc\test 
rm -r .\src\HyperlightGuest\third_party\flatcc\src\cli
rm -r .\src\HyperlightGuest\third_party\flatcc\src\compiler
rm -r .\src\HyperlightGuest\third_party\flatcc\.travis.yml
rm -r .\src\HyperlightGuest\third_party\flatcc\appveyor.yml
rm -r .\src\HyperlightGuest\third_party\flatcc\CMakeLists.txt
rm -r .\src\HyperlightGuest\third_party\flatcc\src\runtime\CMakeLists.txt
rm -r .\src\HyperlightGuest\third_party\flatcc\include\flatcc\support
rm -r .\src\HyperlightGuest\third_party\flatcc\include\flatcc\reflection
 ```
