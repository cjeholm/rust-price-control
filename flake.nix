# Nix flake based developer shell
# Run with: nix develop
# From Vimjoyer's video "The Best Way To Use Python On NixOS"
# https://www.youtube.com/watch?v=6fftiTJ2vuQ
{
  description = "Flake-based dev shell for Rust with Linux + Windows cross-compilation";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    # Prefer rust-bin for reliable multi-target support
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [rust-overlay.overlays.default];
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = [
        (pkgs.rust-bin.stable.latest.default.override {
          targets = ["x86_64-pc-windows-gnu"];
        })

        pkgs.pkgsCross.mingwW64.buildPackages.gcc
        pkgs.pkgsCross.mingwW64.windows.pthreads

        pkgs.openssl
        pkgs.pkg-config
      ];

      RUST_BACKTRACE = 1;
      RUST_LOG = "info";

      shellHook = ''
        echo "Flake-based dev shell for Rust with Linux + Windows cross-compilation"
      '';
    };
  };
}
