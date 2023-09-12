# dummyguest in rs

This is dummyguest, but written in rust.

## x86_64-pc-windows-gnu

### Building

To build, run:

```
RUSTFLAGS="-L/home/dchiarlone/repos/hyperlight/release -C linker-flavor=ld -C link-arg=-T/home/dchiarlone/repos/hyperlight/src/tests/rust_guests/dummyguest/linker_script.ld" cargo build --target x86_64-pc-windows-gnu --release --no-default-features
```

### Pre-reqs

```
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64
```