{
  description = "Basic rust flake (perhaps remove b/c toolchain shenanigans?)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = inputs.nixpkgs.lib.systems.flakeExposed;
      perSystem =
        { lib, pkgs, self', ... }:
        {
          packages.default = self'.packages.cli;

          packages.cli = let
            toml = lib.importTOML ./dos-cli/Cargo.toml;
          in
            pkgs.rustPlatform.buildRustPackage {
              pname = "diff-of-services-cli";
              inherit (toml.package) version;

              # [TODO] fine-grained to avoid changes in other crates
              src = lib.sources.cleanSource ./.;

              cargoLock = {
                lockFile = ./Cargo.lock;
              };

              meta = {
                description = "CLI for diff-of-services, a ToS tracking / diff tool";
                homepage = "https://github.com/amusingimpala75/amusingimpala75/diff-of-services";
                license = lib.licenses.mit;
                mainProgram = toml.package.name;
              };
            };

          devShells.default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              rustc
              rustfmt
            ];
          };
        };
    };
}
