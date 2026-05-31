# SPDX-License-Identifier: AGPL-3.0-only
{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "spotrak";
  version = (lib.importTOML ../Cargo.toml).package.version;
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../db/spotrak-codegen
      ../src
      ../migrations
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;

  # The test suite needs Docker-backed PostgreSQL through testcontainers.
  doCheck = false;

  meta = {
    description = "self-hostable music tracking dashboard for Spotify";
    mainProgram = "spotrak";
    license = lib.licenses.agpl3Only;
  };
}
