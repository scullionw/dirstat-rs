# dirstat-rs

Fast, cross-platform disk usage CLI

[![Crates.io](https://img.shields.io/crates/v/dirstat-rs.svg)](https://crates.io/crates/dirstat-rs)
[![Docs.rs](https://docs.rs/dirstat-rs/badge.svg)](https://docs.rs/dirstat-rs/)
![Language](https://img.shields.io/badge/language-rust-orange)
![Platforms](https://img.shields.io/badge/platforms-Windows%2C%20macOS%20and%20Linux-blue)
![License](https://img.shields.io/github/license/scullionw/dirstat-rs)

![](demo/ds_demo.gif)

2X faster than du

4X faster than ncdu, dutree, dua, du-dust

6X faster than windirstat

(On 4-core hyperthreaded cpu)
        
# Installation

## Homebrew (macOS only)

    brew tap scullionw/tap
    brew install dirstat-rs

## Or if you prefer compiling yourself

### from crates.io:

        cargo install dirstat-rs
        
### or latest from git:

        cargo install --git "https://github.com/scullionw/dirstat-rs"
        
### or from source:

        cargo build --release
        sudo chmod +x /target/release/ds
        sudo cp /target/release/ds /usr/local/bin/

# Usage

### Current directory
    
        $ ds
    
### Specific path
 
        $ ds PATH

### Choose depth
 
        $ ds -d 3

### Show apparent size on disk

        $ ds -a PATH

### Override minimum size threshold

        $ ds -m 0.2 PATH



    
    
    
    
