{
  description = "self-hostable music tracking dashboard for Spotify";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      linuxSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      darwinSystems = [
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      allSystems = linuxSystems ++ darwinSystems;
      forAllSystems =
        systems: f:
        lib.genAttrs systems (
          system: f (nixpkgs.legacyPackages.${system} or (import nixpkgs { inherit system; }))
        );
    in
    {
      packages = forAllSystems linuxSystems (pkgs: rec {
        spotrak = pkgs.callPackage ./nix/package.nix { };
        spotrak-web = pkgs.callPackage ./nix/web.nix { };
        default = spotrak;
      });

      devShells = forAllSystems allSystems (pkgs: {
        default = pkgs.callPackage ./shell.nix { };
      });

      nixosModules.default = import ./nix/nixos.nix;
      darwinModules.default = import ./nix/darwin.nix;
    };
}
