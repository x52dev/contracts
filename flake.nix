{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
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
      perSystem = { pkgs, config, inputs', system, lib, ... }:
        let
          x52just = inputs'.x52.packages.x52-just;
        in
        {
          formatter = pkgs.nixpkgs-fmt;

          devShells.default = pkgs.mkShell {
            buildInputs = [ x52just ];

            packages = [
              config.formatter
              inputs'.nixpkgs-unstable.legacyPackages.cargo-shear
              pkgs.fd
              pkgs.just
              pkgs.nodePackages.prettier
              pkgs.taplo
            ] ++ lib.optional pkgs.stdenv.isDarwin [
              pkgs.pkgsBuildHost.darwin.apple_sdk.frameworks.CoreFoundation
              pkgs.pkgsBuildHost.darwin.apple_sdk.frameworks.Security
              pkgs.pkgsBuildHost.libiconv
            ];

            shellHook = ''
              mkdir -p .toolchain
              cp ${x52just}/*.just .toolchain/
            '';
          };
        };
    };
}
