{
  pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz"),
  ...
}@args:

let
  himalaya-tui = import ./default.nix (
    removeAttrs args [
      "crossPkgs"
      "isStatic"
      "target"
    ]
  );

in
pimalaya.mkDefault (
  {
    src = ./.;
    version = "0.1.0";
    mkPackage = (
      {
        lib,
        pkgs,
        rustPlatform,
        defaultFeatures,
        features,
        buildPackages,
      }:

      let
        inherit (pkgs) sqlite stdenv windows;

        # NOTE: nixpkgs' mingw sqlite fails its pthread probe and compiles
        # single-threaded, defining no sqlite3_mutex_* rusqlite links against
        sqlite' =
          if stdenv.hostPlatform.isWindows then
            sqlite.overrideAttrs (old: {
              buildInputs = (old.buildInputs or [ ]) ++ [ windows.pthreads ];
            })
          else
            sqlite;

        buildFeatures = lib.splitString "," features;

        # NOTE: himalaya's pimdir backend links the system SQLite, as
        # himalaya's own package does.
        systemSqlite =
          (defaultFeatures || builtins.elem "pimdir" buildFeatures)
          && !builtins.elem "vendored" buildFeatures;

      in
      (pkgs.callPackage ./package.nix {
        inherit lib rustPlatform buildFeatures;
        buildPackages = buildPackages // {
          inherit himalaya-tui;
        };
        installShellCompletions = false;
        installManPages = false;
        buildNoDefaultFeatures = !defaultFeatures;
      }).overrideAttrs
        (drv: {
          nativeBuildInputs =
            (drv.nativeBuildInputs or [ ]) ++ lib.optional systemSqlite buildPackages.pkg-config;

          buildInputs = (drv.buildInputs or [ ]) ++ lib.optional systemSqlite sqlite';
        })
    );
  }
  // removeAttrs args [ "pimalaya" ]
)
