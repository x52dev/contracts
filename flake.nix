{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    x52 = {
      url = "github:x52dev/nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };
  };

  outputs = inputs @ { flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      imports = [ inputs.x52.flakeModules.default ];

      perSystem = { pkgs, config, inputs', system, lib, ... }: {
          formatter = pkgs.nixpkgs-fmt;

          devShells.default = pkgs.mkShell {
            buildInputs = [
              inputs'.x52.packages.x52-release-tools
            ];

            packages = [
              config.formatter
              pkgs.cargo-shear
              pkgs.fd
              pkgs.just
              pkgs.prettier
              pkgs.taplo
            ] ++ lib.optional pkgs.stdenv.isDarwin [
              pkgs.pkgsBuildHost.libiconv
            ];

            shellHook = config.x52.justRust.shellHook;
          };
        };
    };
}
