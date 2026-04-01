{
  description = "MP3 Tag Editor - Dioxus webapp";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            wasm-pack
            pkg-config
            openssl
            perl
            cmake
            nodejs
            clang
            lld
            llvmPackages.libclang
            python3
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang}/lib";

          shellHook = ''
            echo "MP3 Tag Editor Dev Shell"
            rustc --version
            rustup target list --installed 2>/dev/null || true
          '';
        };
      }
    );

  nixConfig.experimental-features = [
    "nix-command"
    "flakes"
  ];
}
