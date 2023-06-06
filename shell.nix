# A developement shell for the vrrb repository.
#
# For installation steps see: https://nix.dev/tutorials/install-nix
#
# This dev shell provides some basic developer tools for debugging
# as well as the necessary build dependencies. The rust toolchain
# and rust-analyzer are provided by fenix and `.cargo/bin` is automatically
# added to your $PATH for convenience. For a list of platforms supprted by
# fenix, see: https://github.com/nix-community/fenix#supported-platforms-and-targets
#
# To start the dev shell, simply run: `nix-shell`
#
# By default `nix-shell` opens a new `bash` shell. If you would like to use
# your own, for example `zsh`, you can open a new instance directly after: `zsh`.
# For more information on this limitation see https://nixos.wiki/wiki/Development_environment_with_nix-shell#direnv

{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/nixos-22.11.tar.gz") {} }:

let
  fenix = import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") {};
in
pkgs.mkShell {
  name = "vrrb-dev";

  nativeBuildInputs = [
    # rust toolchain
    (fenix.fromToolchainFile { dir = ./.; })
    pkgs.pkg-config
    pkgs.clang
  ];

  buildInputs = with pkgs; [
    # dev tools
    which
    htop
    zlib

    # build dependencies
    rocksdb
    openssl.dev
    libiconv
  ] ++ lib.optionals stdenv.isDarwin [darwin.apple_sdk.frameworks.Security];

  shellHook = ''
    export LIBCLANG_PATH="${pkgs.libclang.lib}/lib";
    export ROCKSDB_LIB_DIR="${pkgs.rocksdb}/lib";
  '';
}
