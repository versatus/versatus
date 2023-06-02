# A developement shell for the vrrb repository.
#
# For installation steps see: https://nix.dev/tutorials/install-nix
#
# This dev shell provides some basic developer tools for debugging
# as well as the necessary build dependencies. The toolchain path 
# should work for _most_ linux distros and `.cargo/bin` is automatically
# added to your $PATH for convenience.
#
# To start a new instance, simply run: `nix-shell`

{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/nixos-22.11.tar.gz") {} }:

pkgs.mkShell rec {
  name = "vrrb-dev";

  buildInputs = with pkgs; [
    # dev tools
    which
    htop
    zlib

    # build dependencies
    clang
    libclang.lib
    rocksdb
    openssl.dev
    pkg-config
    rustup
  ];

  RUSTC_VERSION = pkgs.lib.readFile ./rust-toolchain.toml;

  shellHook = ''
    export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
    export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
    export LIBCLANG_PATH="${pkgs.libclang.lib}/lib";
    export ROCKSDB_LIB_DIR="${pkgs.rocksdb}/lib";
  '';
}