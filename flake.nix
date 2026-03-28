{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "clippy"
            "rust-analyzer"
            "rust-src"
            "rustfmt"
          ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = pkgs.lib.fileset.toSource {
          root = ./.;
          fileset = pkgs.lib.fileset.unions [
            (craneLib.fileset.commonCargoSources ./.)
            (pkgs.lib.fileset.fileFilter (file: file.hasExt "mid") ./.)
            (pkgs.lib.fileset.fileFilter (file: file.hasExt "snap") ./.)
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly { inherit src; };
      in
      {
        checks = {
          formatting = craneLib.cargoFmt { inherit src; };
          linting = craneLib.cargoClippy {
            inherit src cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          };
          testing = craneLib.cargoTest {
            inherit src cargoArtifacts;
            INSTA_WORKSPACE_ROOT = ".";
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.bacon
            rustToolchain
          ];
        };
      }
    );
}
