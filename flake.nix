{

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    {
      self,
      nixpkgs,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      forEachSystem = lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
      pkgsForEach = nixpkgs.legacyPackages;
    in
    {
      packages = forEachSystem (system: rec {
        default = capport;
        capport = pkgsForEach.${system}.callPackage ./package.nix { };
      });

      devShells = forEachSystem (system: {
        default = pkgsForEach.${system}.callPackage ./shell.nix { };
      });

      nixosModules = {
        default = self.nixosModules.capport;
        capport = ./nix/module.nix;
      };
    };
}
