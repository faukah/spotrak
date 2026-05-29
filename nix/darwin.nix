# SPDX-License-Identifier: AGPL-3.0-only
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.spotrak;
  inherit (lib)
    mkIf
    escapeShellArg
    concatStringsSep
    mapAttrsToList
    getExe
    ;

  exports = concatStringsSep "\n" (
    mapAttrsToList (k: v: "export ${k}=${escapeShellArg v}") (
      {
        DATABASE_URL = cfg.database.url;
        API_ENDPOINT = cfg.apiEndpoint;
        CLIENT_ENDPOINT = cfg.clientEndpoint;
        CORS = cfg.cors;
        SPOTIFY_PUBLIC = cfg.spotifyPublic;
        PORT = toString cfg.backendPort;
        IMPORT_DIR = "${cfg.stateDir}/imports";
      }
      // cfg.settings
    )
  );

  backendScript = pkgs.writeShellScript "spotrak-serve" ''
    set -a; . ${escapeShellArg (toString cfg.environmentFile)}; set +a
    ${exports}
    mkdir -p ${escapeShellArg "${cfg.stateDir}/imports"}
    exec ${getExe cfg.package} serve
  '';

  webScript = pkgs.writeShellScript "spotrak-web-serve" ''
    export HOST=127.0.0.1 PORT=${toString cfg.webPort}
    export SERVER_API_ENDPOINT=http://127.0.0.1:${toString cfg.backendPort}
    export HOME=/var/lib/spotrak-web DENO_DIR=/var/lib/spotrak-web/deno DENO_NO_UPDATE_CHECK=1
    mkdir -p "$DENO_DIR"
    cd ${cfg.webPackage}
    exec ${pkgs.deno}/bin/deno run -A --cached-only web/server.ts
  '';
in
{
  imports = [ ./options.nix ];

  config = mkIf cfg.enable {
    launchd.daemons.spotrak.serviceConfig = {
      ProgramArguments = [ "${backendScript}" ];
      KeepAlive = true;
      RunAtLoad = true;
      UserName = cfg.user;
      StandardErrorPath = "/var/log/spotrak.log";
      StandardOutPath = "/var/log/spotrak.log";
    };

    launchd.daemons.spotrak-web.serviceConfig = {
      ProgramArguments = [ "${webScript}" ];
      KeepAlive = true;
      RunAtLoad = true;
      UserName = cfg.user;
      StandardErrorPath = "/var/log/spotrak-web.log";
      StandardOutPath = "/var/log/spotrak-web.log";
    };
  };
}
