# unikraft-rs (static library)

> [!IMPORTANT]  
> This approach is deprecated and will not work on current Unikraft versions.
> See [`*-unikraft-linux-musl`—The rustc book](https://doc.rust-lang.org/rustc/platform-support/unikraft-linux-musl.html) for running Rust apps on current versions of Unikraft.

This crate builds and links against [Unikraft].

[Unikraft]: https://github.com/unikraft/unikraft

## Requirements

* [KraftKit](https://github.com/unikraft/kraftkit)
* [Rust](https://www.rust-lang.org/tools/install)
    * Either [Rust for Unikraft](https://github.com/unikraft/rust) (`x86_64-unikraft`)
    * Or Rust nightly (`x86_64-unknown-linux-gnu`, `no-std` only)

## Supported Unikraft platforms

* `kvm`
* `linuxu`

## Usage

You can compile the [examples] like this:

```console
cargo build \
    --example <EXAMPLE> \
    --features <PLATFORM> \
    --target <TRIPLE>
```

[examples]: examples

### `x86_64-unknown-linux-gnu`

You can only build `no-std` applications using the `x86_64-unknown-linux-gnu` target.

This target requires additional `RUSTFLAGS`:

```console
-Crelocation-model=static
-Clink-arg=-Wl,-T,unikraft_linker_script.ld
-Clink-arg=-Wl,-dT,default_unikraft_linker_script.ld
-Clink-arg=-Wl,-e,_unikraft_rs_start
-Cpanic=abort
```

See [Cargo configuration `build.rustflags`] and [cargo-rustc] for more information on how to set them.

[Cargo configuration `build.rustflags`]: https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags
[cargo-rustc]: https://doc.rust-lang.org/cargo/commands/cargo-rustc.html

You also need to provide stubs for `eh_personality` and `_Unwind_Resume` as seen in the [example].

[example]: examples/no-std.rs

## License

unikraft-rs is part of the [Unikraft OSS Project][unikraft-website] and licensed under `BSD-3-Clause`.

[unikraft-website]: https://unikraft.org
