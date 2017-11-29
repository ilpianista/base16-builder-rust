# Base16 Builder Rust

[![Build Status](https://gitlab.com/ilpianista/base16-builder-rust/badges/master/build.svg)](https://gitlab.com/ilpianista/base16-builder-rust/pipelines)

A Rust implementation of a base16 builder that follows the conventions described at https://github.com/chriskempson/base16.

Version 0.9.0.

**This is WIP!**

It does not update existing schemes and existing templates repositories, but knows how to perform a full build.

## Installation

### From sources

    git clone https://github.com/ilpianista/base16-builder-rust
    cd base16-builder-rust
    cargo build
    cargo run

## Usage

Execute `cargo run` or `base16-builder-rust` to build all templates using all schemes.
