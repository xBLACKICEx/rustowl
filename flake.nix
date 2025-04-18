{
  description = "RustOwl Nix/NixOS Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rustowl/rust-toolchain.toml;

        rustToolchainToml = builtins.fromTOML (builtins.readFile ./rustowl/rust-toolchain.toml);
        cargoToml = builtins.fromTOML (builtins.readFile ./rustowl/Cargo.toml);

        envAttrs = {
          RUSTOWL_TOOLCHAIN = rustToolchainToml.toolchain.channel;
          RUSTOWL_TOOLCHAIN_DIR = "${rustToolchain}";
        };

        rustowl = pkgs.rustPlatform.buildRustPackage ({
          pname = "rustowl";
          version = cargoToml.package.version;
          src = ./rustowl;

          cargoLock.lockFile = ./rustowl/Cargo.lock;

          nativeBuildInputs = [ rustToolchain ];

          buildInputs = [ ];

          meta = with pkgs.lib; {
            description = "Visualize ownership and lifetimes in Rust for debugging and optimization";
            longDescription = ''
              RustOwl visualizes ownership movement and lifetimes of variables.
              When you save Rust source code, it is analyzed, and the ownership and
              lifetimes of variables are visualized when you hover over a variable or function call.
            '';
            homepage = "https://github.com/cordx56/rustowl";
            license = licenses.mpl20;
            platforms = platforms.all;
          };
        }
        // envAttrs);
      in
      {
        packages = {
          default = rustowl;
        };

        devShells.default = pkgs.mkShell ({
          buildInputs = [
            (rustToolchain.override {
              extensions = [ "rust-src" "rustfmt" ];
            })
            # rustowl
          ];

          shellHook = ''
            echo "RustOwl development loaded!"
            echo "Using toolchain: ${envAttrs.RUSTOWL_TOOLCHAIN}"
          '';
        }
        // envAttrs);
      }
    );
}
