{
  description = "Binary blobs with intelligent serialization";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.flake-compat.url = "github:edolstra/flake-compat";
  inputs.flake-compat.flake = false;

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  inputs.pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  inputs.pre-commit-hooks.inputs.flake-utils.follows = "flake-utils";
  inputs.pre-commit-hooks.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, flake-utils, rust-overlay, pre-commit-hooks, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [
          (import rust-overlay)
        ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.minimal.override {
          extensions = [ "rustfmt" "clippy" "llvm-tools-preview" "rust-src" ];
        };
        rustDeps = with pkgs;
          [
            pkg-config
            openssl
            bash

            curl
            docker

            cargo-audit
            cargo-edit
            cargo-udeps
            cargo-sort
            cmake
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.SystemConfiguration

            # https://github.com/NixOS/nixpkgs/issues/126182
            libiconv
          ] ++ lib.optionals (!stdenv.isDarwin) [
            cargo-watch # broken: https://github.com/NixOS/nixpkgs/issues/146349
          ];
        # nixWithFlakes allows pre v2.4 nix installations to use
        # flake commands (like `nix flake update`)
        nixWithFlakes = pkgs.writeShellScriptBin "nix" ''
          exec ${pkgs.nixFlakes}/bin/nix --experimental-features "nix-command flakes" "$@"
        '';
        shellHook  = ''
          # Prevent cargo aliases from using programs in `~/.cargo` to avoid conflicts with rustup
          # installations.
          export CARGO_HOME=$HOME/.cargo-nix
          export PATH="$PWD/$CARGO_TARGET_DIR/release:$PATH"
        '';

        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        RUST_BACKTRACE = 1;
        RUST_LOG = "info";
        # Use a distinct target dir for builds from within nix shells.
        CARGO_TARGET_DIR = "target/nix";
      in {
      	checks = {
          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              cargo-fmt = {
                enable = true;
                description = "Enforce rustfmt";
                entry = "cargo fmt --all -- --check";
                pass_filenames = false;
              };
              cargo-sort = {
                enable = true;
                description = "Ensure Cargo.toml are sorted";
                entry = "cargo sort -g -w";
                pass_filenames = false;
              };
              cargo-clippy = {
                enable = true;
                description = "Run clippy";
                entry = "cargo clippy --workspace --all-features --all-targets -- -D clippy::dbg-macro";
                pass_filenames = false;
              };
            };
          };
        };
        devShell = pkgs.mkShell {
          shellHook = shellHook
          	# install pre-commit hooks
            + self.checks.${system}.pre-commit-check.shellHook;
          buildInputs = with pkgs;
            [
              rust-bin.nightly.latest.rust-analyzer
              nixWithFlakes
              nixpkgs-fmt
              git
              rustToolchain
            ] ++ rustDeps;

          inherit RUST_SRC_PATH RUST_BACKTRACE RUST_LOG CARGO_TARGET_DIR;
        };
      });
}
