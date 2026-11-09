{
  description = "wasm-lang: a small compiler (nom parser + three-address IR + 32-bit x86 backend)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        inherit (pkgs) lib stdenv;

        runtimeTools = lib.optionals stdenv.isLinux [
          pkgs.nasm
          pkgs.qemu
          pkgs.pkgsi686Linux.stdenv.cc # gcc that links 32-bit binaries
        ];

        cli = pkgs.rustPlatform.buildRustPackage {
          pname = "wasm-lang";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = {
            description = "Compiler CLI for a small made-up language";
            mainProgram = "cli";
          };
        };
      in
      {
        packages.default = cli;
        packages.cli = cli;

        apps.default = flake-utils.lib.mkApp {
          drv = cli;
          name = "cli";
        };

        devShells.default = pkgs.mkShell ({
          packages = [
            pkgs.cargo
            pkgs.rustc
            pkgs.clippy
            pkgs.rustfmt
            pkgs.rust-analyzer
          ] ++ runtimeTools;

          shellHook = ''
            echo "wasm-lang dev shell"
            echo "  build:  cargo build"
            echo "  run:    cargo run -p cli -- test/test1 -v"
            echo "  test:   cargo test -p parser         (unit tests, any platform)"
          '';
        } // lib.optionalAttrs stdenv.isLinux {
          CC32 = "${pkgs.pkgsi686Linux.stdenv.cc}/bin/cc";
          QEMU = "qemu-i386";
        });

        checks.build = cli;
      });
}
