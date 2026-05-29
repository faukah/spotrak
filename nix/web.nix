# SPDX-License-Identifier: AGPL-3.0-only
{
  lib,
  stdenv,
  deno,
  nukeReferences,
  cacert,
  apiEndpoint ? "http://localhost:8080",
}:
let
  version = (lib.importTOML ../Cargo.toml).package.version;
  astro = "npm:astro@6.3.6";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../web
      ../deno.json
      ../deno.lock
    ];
  };

  deps = stdenv.mkDerivation {
    name = "spotrak-web-deps";
    inherit src;
    nativeBuildInputs = [
      deno
      nukeReferences
    ];
    env.SSL_CERT_FILE = "${cacert}/etc/ssl/certs/ca-bundle.crt";
    env.DENO_NO_UPDATE_CHECK = "1";
    dontConfigure = true;
    # Fixed-output derivations cannot retain store references from patched
    # vendored scripts or prebuilt native module fixups.
    dontFixup = true;
    buildPhase = ''
      runHook preBuild
      export HOME=$TMPDIR DENO_DIR=$TMPDIR/deno-dir
      deno install --frozen
      runHook postBuild
    '';
    installPhase = ''
      runHook preInstall
      rm -f node_modules/.deno/.setup-cache.bin
      find node_modules -type d -name .bin -exec rm -rf {} +
      mkdir -p $out
      cp -r node_modules $out/node_modules
      chmod -R u+w $out
      find $out -type l | while read -r l; do
        case "$(readlink "$l")" in *nix/store*) rm -f "$l" ;; esac
      done
      find $out -type f -exec nuke-refs {} +
      runHook postInstall
    '';
    outputHashMode = "recursive";
    outputHashAlgo = "sha256";
    outputHash = "sha256-jVS/8emWPwjUFnAWZQh/E+JLN3bLGb3yq/KZ/D71IQQ=";
  };
in
stdenv.mkDerivation {
  pname = "spotrak-web";
  inherit version src;
  nativeBuildInputs = [ deno ];
  env.DENO_NO_UPDATE_CHECK = "1";
  env.PUBLIC_API_ENDPOINT = apiEndpoint;
  dontConfigure = true;
  dontFixup = true;
  buildPhase = ''
    runHook preBuild
    export HOME=$TMPDIR DENO_DIR=$TMPDIR/deno-dir
    cp -r ${deps}/node_modules node_modules && chmod -R u+w node_modules
    deno run -A --cached-only ${astro} build --root web
    runHook postBuild
  '';
  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r web $out/web
    cp -r node_modules $out/node_modules
    cp deno.json deno.lock $out/
    runHook postInstall
  '';
  passthru.deps = deps;
  meta = {
    description = "spotrak Astro SSR frontend (Deno)";
    license = lib.licenses.agpl3Only;
  };
}
