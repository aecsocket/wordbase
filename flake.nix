{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        # TODO: merge with `fromRustupToolchainFile ./rust-toolchain` somehow?
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [
            "rustc"
            "cargo"
            "rustfmt"
            "rust-std"
            "rust-docs"
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustc-codegen-cranelift-preview"
          ];
        });
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            just
            fish
            typos

            # Nix
            nixd
            nil
            nixfmt-rfc-style
            nixfmt-tree

            # Rust
            rustToolchain
            taplo
            cargo-shear
            pkg-config
            openssl
            sqlx-cli
            sqlite

            # Binding generation
            ktlint
          ];
          shellHook = ''
            export RUSTFLAGS="-Zcodegen-backend=cranelift"
          '';
        };
      }
    );
}
