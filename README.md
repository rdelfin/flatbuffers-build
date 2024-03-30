# flatbuffers-gen

This crate provides a set of functions to facilitate compiling flatbuffers to Rust from within
Rust. This is particularly helpful for use in `build.rs` scripts. Please note that for
compatiblity this crate will only support a single version of the `flatc` compiler. Please
check what version that is against whatever version is installed on your system.That said, due
to flatbuffers' versioning policy, it could be ok to mix patch and even minor versions.

If you're not sure where to start, take a look at [`BuilderOptions`]. Please also look at the
[`flatbuffers-example`](https://github.com/rdelfin/flatbuffers-gen/tree/main/flatbuffers-example)
folder in the repo for an example. However, we'll explain the full functionality here.

As an example, imagine a crate with the following folder structure:
```bash
├── build.rs
├── Cargo.toml
├── example.fbs
└── src
    └── lib.rs
```
In order to compile and use the code generated from `example.fbs` code, first you need to add
`flatbuffers-gen` to your build dependencies, as well as a matching version of `flatbuffers`:
```toml
# Cargo.toml
# [...]
[dependencies]
flatbuffers = "=23.5.26"

[build-dependencies]
flatbuffers-gen = "=23.5.26"
# [...]
```

You can then have a very simple `build.rs` as follows:
```no_run
use flatbuffers_gen::BuilderOptions;

BuilderOptions::new_with_files(["example.fbs"])
    .compile()
    .expect("flatbuffer compilation failed");
```

Note here that `example.fbs` is the same one provided by `flatbuffers` as an example. The
namespace is `MyGame.Sample` and it contains multiple tables and structs, including a `Monster`
table.

This will just compile the flatbuffers and drop them in `OUT_DIR`. You can then pull them in in
`lib.rs` like so:

```rust,ignore
#[allow(warnings)]
pub mod defs {
    include!(concat!(env!("OUT_DIR"), "/example_generated.rs"));
}

use defs::my_game::sample::Monster;

fn some_fn() {
    // Make use of `Monster`
}
```
