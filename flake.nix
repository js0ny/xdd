{
  description = "Cross-platform directory definition URL handler";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          lib = pkgs.lib;
          buildPackage =
            packageSet:
            let
              desktopItems = lib.optionals packageSet.stdenv.hostPlatform.isLinux [
                (packageSet.makeDesktopItem {
                  name = "xdd";
                  desktopName = "xdd";
                  exec = "xdd open %u";
                  noDisplay = true;
                  terminal = false;
                  mimeTypes = [ "x-scheme-handler/xdd" ];
                })
              ];
            in
            packageSet.rustPlatform.buildRustPackage {
              pname = manifest.package.name;
              version = manifest.package.version;
              src = lib.cleanSource ./.;
              cargoLock.lockFile = ./Cargo.lock;
              nativeBuildInputs = lib.optionals packageSet.stdenv.hostPlatform.isLinux [
                packageSet.copyDesktopItems
              ];
              inherit desktopItems;

              meta = {
                description = "Cross-platform directory definition URL handler";
                license = lib.licenses.gpl3Plus;
                platforms = with lib.platforms; linux ++ darwin;
                sourceProvenance = [ lib.sourceTypes.fromSource ];
              };
            };
        in
        {
          default = buildPackage pkgs;
        }
        // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          musl = buildPackage pkgs.pkgsStatic;
        }
      );
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          ciDeps = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
          ];
          devDeps = with pkgs; [ rust-analyzer ];
        in
        {
          default = pkgs.mkShell {
            buildInputs = ciDeps ++ devDeps;
          };
        }
      );
    };
}
