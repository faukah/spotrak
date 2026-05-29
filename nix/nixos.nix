# SPDX-License-Identifier: AGPL-3.0-only
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.spotrak;
  inherit (lib) mkDefault mkIf optionals;
  forwardedHeaders = ''
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
  '';
  webDeno = "${pkgs.deno}/bin/deno run -A --cached-only web/server.ts";
in
{
  imports = [ ./options.nix ];

  config = mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home = cfg.stateDir;
    };
    users.groups.${cfg.group} = { };

    systemd.tmpfiles.rules = [
      "d ${cfg.stateDir} 0750 ${cfg.user} ${cfg.group} -"
      "d ${cfg.stateDir}/imports 0750 ${cfg.user} ${cfg.group} -"
      "d /var/lib/spotrak-web 0750 ${cfg.user} ${cfg.group} -"
    ];

    services.postgresql = mkIf cfg.database.createLocally {
      enable = true;
      ensureDatabases = [ "spotrak" ];
      ensureUsers = [
        {
          name = cfg.user;
          ensureDBOwnership = true;
          ensureClauses.login = true;
        }
      ];
    };

    services.nginx = mkIf cfg.nginx.enable {
      enable = true;
      virtualHosts.${cfg.nginx.hostName} = {
        forceSSL = mkDefault true;
        enableACME = mkDefault true;
        locations = {
          "/api/" = {
            proxyPass = "http://127.0.0.1:${toString cfg.backendPort}";
            extraConfig = forwardedHeaders;
          };

          "/" = {
            root = "${cfg.webPackage}/web/dist/client";
            tryFiles = "$uri @ssr";
          };

          "@ssr" = {
            proxyPass = "http://127.0.0.1:${toString cfg.webPort}";
            proxyWebsockets = true;
            extraConfig = forwardedHeaders;
          };
        };
      };
    };

    systemd.services.spotrak = {
      description = "spotrak backend (axum + postgres)";
      wantedBy = [ "multi-user.target" ];
      wants = [ "network-online.target" ];
      after = [
        "network-online.target"
      ]
      ++ optionals cfg.database.createLocally [ "postgresql.service" ];
      requires = optionals cfg.database.createLocally [ "postgresql.service" ];

      environment = {
        DATABASE_URL = cfg.database.url;
        API_ENDPOINT = cfg.apiEndpoint;
        CLIENT_ENDPOINT = cfg.clientEndpoint;
        CORS = cfg.cors;
        SPOTIFY_PUBLIC = cfg.spotifyPublic;
        PORT = toString cfg.backendPort;
        IMPORT_DIR = "${cfg.stateDir}/imports";
      }
      // cfg.settings;

      serviceConfig = {
        User = cfg.user;
        Group = cfg.group;
        EnvironmentFile = cfg.environmentFile;
        WorkingDirectory = cfg.stateDir;
        ExecStart = "${lib.getExe cfg.package} serve";
        Restart = "on-failure";
        RestartSec = 5;
      };
    };

    systemd.services.spotrak-web = {
      description = "spotrak web frontend (astro SSR on deno)";
      wantedBy = [ "multi-user.target" ];
      wants = [ "network-online.target" ];
      after = [
        "network-online.target"
        "spotrak.service"
      ];

      environment = {
        HOST = "127.0.0.1";
        PORT = toString cfg.webPort;
        SERVER_API_ENDPOINT = "http://127.0.0.1:${toString cfg.backendPort}";
        HOME = "/var/lib/spotrak-web";
        DENO_DIR = "/var/lib/spotrak-web/deno";
        DENO_NO_UPDATE_CHECK = "1";
      };

      serviceConfig = {
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = cfg.webPackage;
        ExecStart = webDeno;
        Restart = "on-failure";
        RestartSec = 5;
      };
    };
  };
}
