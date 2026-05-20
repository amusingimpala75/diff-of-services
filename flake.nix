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

              cargoBuildFlags = [ "--bin" "dos-cli" ];

              nativeBuildInputs = [ pkgs.installShellFiles ];

              postInstall = ''
                $out/bin/dos-cli util generate manpages
                installManPage man/*.1

                $out/bin/dos-cli util generate shell-completions
                installShellCompletion \
                  --bash completions/dos.bash \
                  --zsh completions/_dos \
                  --fish completions/dos.fish
              '';

              cargoTestFlags = [ "-p" "dos-lib" "-p" "dos-cli" ];

              meta = {
                description = "CLI for diff-of-services, a ToS tracking / diff tool";
                homepage = "https://github.com/amusingimpala75/amusingimpala75/diff-of-services";
                license = lib.licenses.mit;
                mainProgram = toml.package.name;
              };
            };

          packages.tui = let
            toml = lib.importTOML ./dos-tui/Cargo.toml;
          in
            pkgs.rustPlatform.buildRustPackage {
              pname = "diff-of-services-tui";
              inherit (toml.package) version;
              src = lib.sources.cleanSource ./.;

              cargoLock = {
                lockFile = ./Cargo.lock;
              };

              cargoBuildFlags = [ "--bin" "dos-tui" ];

              cargoTestFlags = [ "-p" "dos-lib" "-p" "dos-tui" ];

              meta = {
                description = "TUI for diff-of-services, a ToS tracking / diff tool";
                homepage = "https://github.com/amusingimpala75/amusingimpala75/diff-of-services";
                license = lib.licenses.mit;
                mainProgram = toml.package.name;
              };
            };

          packages.gui = let
            toml = lib.importTOML ./dos-gui/Cargo.toml;
          in
            pkgs.rustPlatform.buildRustPackage {
              pname = "diff-of-services-gui";
              inherit (toml.package) version;
              src = lib.sources.cleanSource ./.;

              cargoLock.lockFile = ./Cargo.lock;

              pnpmDeps = pkgs.fetchPnpmDeps {
                pname = "diff-of-services-pnpm-deps";
                inherit (toml.package) version;
                src = lib.cleanSource ./dos-gui/frontend;
                fetcherVersion = 3;
                hash = "sha256-NYOJVWoTKdgCqtH+XlL06UBjdpJ9Mr6z9q1kQc+gGUY=";
              };

              pnpmRoot = "dos-gui/frontend";

              preBuild = ''
                cargo tauri icon icon.png --output dos-gui/icons
              '';

              nativeBuildInputs = with pkgs; [
                cargo-tauri.hook

                nodejs
                pnpmConfigHook
                pnpm

                pkg-config
              ] ++ lib.optionals stdenv.hostPlatform.isLinux [ wrapGAppsHook4 ];

              buildInputs = lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.webkitgtk_4_1 ];

              cargoTestFlags = [ "-p" "dos-lib" "-p" "dos-gui" ];
            };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ self'.packages.cli self'.packages.tui self'.packages.gui ];
            packages = with pkgs; [
              clippy
              rustfmt
            ];
          };
          devShells.ios = lib.mkIf pkgs.stdenv.hostPlatform.isDarwin (pkgs.mkShell {
            inputsFrom = [ self'.devShells.default ];
            packages = with pkgs; [
              cocoapods
              iconv
              libimobiledevice
              llvmPackages.clang-unwrapped
              rustup
              xcodegen
            ];
          });
        };
    };
}
