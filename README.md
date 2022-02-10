## libjuice-rs [![latest]][crates.io] [![doc]][docs.rs]

[latest]: https://img.shields.io/crates/v/libjuice-rs.svg
[crates.io]: https://crates.io/crates/libjuice-rs
[doc]: https://docs.rs/libjuice-rs/badge.svg
[docs.rs]: https://docs.rs/libjuice-rs

Rust bindings for [libjuice](https://github.com/paullouisageneau/libjuice).
Look at [datachannel-rs](https://github.com/lerouxrgd/datachannel-rs) if you need more batteries.

### Usage
Please refer to [tests](https://github.com/VollmondT/juice-rs/blob/main/tests/connectivity.rs), 
also refer to the original library [tests](https://github.com/paullouisageneau/libjuice/blob/master/test/connectivity.c).

### Building
Currently, only static linking with the [libjuice](https://github.com/paullouisageneau/libjuice)
is supported.

You need to have:
* [CMake](https://cmake.org/)
* [libclang](https://clang.llvm.org/) (for bindgen)

Clone repository recursively:

`$ git clone https://github.com/VollmondT/juice-rs.git --recursive`

Play with tests:

```
$ cd juice-rs
$ cargo test
```