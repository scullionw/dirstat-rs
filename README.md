# dirstat-rs

[![Crates.io](https://img.shields.io/crates/v/dirstat-rs.svg)](https://crates.io/crates/dirstat-rs)
[![Docs.rs](https://docs.rs/dirstat-rs/badge.svg)](https://docs.rs/dirstat-rs/)

2X faster than du

4X faster than ncdu, dutree, dua, du-dust

6X faster than windirstat


(On 4-core hyperthreaded cpu)


    A disk usage cli similar to windirstat

    USAGE:
        ds [OPTIONS] [target_dir]

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    OPTIONS:
        -d <max_depth>          Maximum recursion depth in directory [default: 1]
        -m <min_percent>        Threshold that determines if entry is worth being shown. Between 0-100 % of dir size.
                                [default: 1]

    ARGS:
        <target_dir>
