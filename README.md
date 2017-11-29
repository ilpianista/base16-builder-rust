# Base16 Builder Rust

[![Build Status](https://gitlab.com/ilpianista/base16-builder-rust/badges/master/build.svg)](https://gitlab.com/ilpianista/base16-builder-rust/pipelines)

A Rust implementation of a base16 builder that follows the conventions described at https://github.com/chriskempson/base16.

Version 0.9.0.

## Installation

### From sources

    git clone https://github.com/ilpianista/base16-builder-rust
    cd base16-builder-rust
    cargo build

## Usage

    target/debug/base16-builder update
Updates all scheme and template repositories as defined in `schemes.yaml` and `templates.yaml`.

    target/debug/base16-builder
Build all templates using all schemes
