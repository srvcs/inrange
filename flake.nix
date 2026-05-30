{
  description = "srvcs-inrange: range: is value within [lo, hi]";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        version = "0.1.0";
        rustToolchain = pkgs.rust-bin.stable."1.96.0".default.override {
          extensions = [ "clippy" "rustfmt" ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
        packages = {
          default = rustPlatform.buildRustPackage {
            pname = "srvcs-inrange";
            inherit version;
            src = ./.;
            cargoHash = "sha256-vIcFa4L6Mq6Q6MjhbIzCL1QVXZYv0PWyI3itwMl47wU=";
          };
        } // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          container = pkgs.dockerTools.buildLayeredImage {
            name = "srvcs-inrange";
            tag = "latest";
            config = {
              Entrypoint = [ "${self.packages.${system}.default}/bin/srvcs-inrange" ];
              ExposedPorts = { "8080/tcp" = { }; };
              User = "65534:65534";
              Labels = {
                "org.opencontainers.image.title" = "srvcs-inrange";
                "org.opencontainers.image.description" = "Range orchestrator: a thin alias of srvcs-between, delegating whether value is within [lo, hi] entirely to srvcs-between.";
                "org.opencontainers.image.version" = version;
                "org.opencontainers.image.revision" = self.rev or "dev";
                "org.opencontainers.image.source" = "https://github.com/srvcs/inrange";
                "org.opencontainers.image.licenses" = "Apache-2.0";
              };
            };
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [ rustToolchain pkgs.syft ];
        };
      });
}
