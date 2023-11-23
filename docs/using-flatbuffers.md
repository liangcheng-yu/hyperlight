# Using FlatBuffers

Flatbuffers is used to serialise and deserialise some data structures.

Schema files are used to define the data structures and are used to generate the code to serialise and deserialise the data structures.

Those files are located in the [`schema`](../src/schema) directory.

Code generated from the schema files is checked in to the repository , therefore you only need to generate the code if you change an existing schema file or add a new one. You can find details on how to update schema files [here](https://google.github.io/flatbuffers/flatbuffers_guide_writing_schema.html).

## Generating code

Two tools are required to generate the code:

* [flatc](https://google.github.io/flatbuffers/flatbuffers_guide_using_schema_compiler.html) - the FlatBuffers schema compiler for all languages except C.
* [flatcc](https://github.com/dvidelabs/flatcc) - the FlatBuffers schema compiler for C.

Follow instructions in the links above to build/or install the tools. You can use vcpkg via `just install-flatbuffers-with-vcpkg:` to install flatc, this will install vcpkg and then install flatc which can then be found at `..\vcpkg\installed\x64-windows\tools\flatbuffers\flatc`  on Windows and `../vcpkg/installed/x64-linux/tools/flatbuffers/flatc` on Linux.. Note that this may not be the latest version.

Once you have the tools installed, you can generate the code by running as follows:

## Linux commands

<details>
<summary>Expand for commands</summary>

### Generate Rust code

```console
flatc -r --rust-module-root-file --gen-all -o ./src/hyperlight_flatbuffers/src/flatbuffers/ ./src/schema/guest_error.fbs 
```

### Generate C# code

```console
flatc -n  --gen-object-api -o ./src/Hyperlight/flatbuffers  ./src/schema/guest_error.fbs
```

</details>

---

## Windows commands
<details>
<summary>Expand for commands</summary>

### Generate Rust code

```console
flatc -r --rust-module-root-file --gen-all -o .\src\hyperlight_flatbuffers\src\flatbuffers\ .\src\schema\guest_error.fbs 
```

### Generate C# code

```console
flatc -n  --gen-object-api -o .\src\Hyperlight\flatbuffers  .\src\schema\guest_error.fbs
```

</details>

---

<br />

### Note about generated Rust code

When generating the Rust code, a `mod.rs` file will be generated in `./src/hyperlight_flatbuffers/src/flatbuffers`, but don't use it. This file will only contain module definitions for the types in the schema file passed as an argument (and any included schema files). If you use this file, you will overwrite existing module definitions for other types previously generated from flatbuffers.

Instead, manually update `./src/hyperlight_flatbuffers/src/flatbuffers/mod.rs` with details of new modules. Whilst `flatc` does support passing multiple schema files (e.g. it is possible to pass `.\src\schema\*.fbs`), so we could regenerate all the files each time a change was made, that functionality does not generate the correct code ( see [here](https://github.com/google/flatbuffers/issues/6800) for details).
