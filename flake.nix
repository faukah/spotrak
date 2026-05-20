{

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    {
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
      devShells = forEachSystem (system: {
        default = pkgsForEach.${system}.callPackage ./shell.nix { };
      });
    };
}
