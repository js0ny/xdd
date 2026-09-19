{
  description = "";
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
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          lib = pkgs.lib;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "diranchor";
            version = "0.1.0";
            src = lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "";
              license = lib.licenses.asl20;
              platforms = with lib.platforms; linux ++ darwin;
            };
          };
        }
      );
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          ciDeps = with pkgs; [
            rustc
            cargo
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
