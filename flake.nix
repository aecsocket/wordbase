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
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            just
            typos

            # Nix
            nixd
            nil
            nixfmt-rfc-style

            # Rust
            rustToolchain
            taplo
            cargo-shear
            pkg-config
            rust-bindgen

            # Wordbase
            openssl
            sqlx-cli
            sqlite

            # GTK app
            gtk4
            libadwaita
            webkitgtk_6_0
            pipewire

            # GNOME extension
            vtsls
            eslint
            gnome-extensions-cli

            # Dioxus
            dioxus-cli
            wasm-bindgen-cli_0_2_100

            # Binding generation
            ktlint
          ];
          nativeBuildInputs = with pkgs; [ clang ];
          LIBCLANG_PATH = with pkgs; lib.makeLibraryPath [ libclang ];
          RUSTFLAGS = "-Zcodegen-backend=cranelift";
          RUST_BACKTRACE = "full";
          shellHook = ''
            mkdir -p "$XDG_DATA_HOME/wordbase"
            export LINDERA_CACHE="$XDG_DATA_HOME/lindera"
            export DATABASE_URL="sqlite://$XDG_DATA_HOME/wordbase/wordbase.db"
          '';
        };
      }
    );
}
