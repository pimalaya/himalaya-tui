{
  nixpkgs ? <nixpkgs>,
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
  ...
}@args:

pimalaya.mkDefault (
  {
    src = ./.;
    version = "0.1.0";
    mkPackage = (
      {
        lib,
        pkgs,
        buildPackages,
        rustPlatform,
        defaultFeatures,
        features,
      }:
      pkgs.callPackage ./package.nix {
        inherit lib rustPlatform buildPackages;
        installShellCompletions = false;
        installManPages = false;
        buildNoDefaultFeatures = !defaultFeatures;
        buildFeatures = lib.splitString "," features;
      }
    );
  }
  // removeAttrs args [ "pimalaya" ]
)
