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
    mkEnableOption
    mkOption
    types
    literalExpression
    ;
in
{
  options.services.spotrak = {
    enable = mkEnableOption "spotrak, a self-hostable Spotify tracking dashboard";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ./package.nix { };
      defaultText = literalExpression "pkgs.callPackage ./package.nix { }";
      description = "The spotrak backend package.";
    };

    webPackage = mkOption {
      type = types.package;
      default = pkgs.callPackage ./web.nix { inherit (cfg) apiEndpoint; };
      defaultText = literalExpression "pkgs.callPackage ./web.nix { inherit (config.services.spotrak) apiEndpoint; }";
      description = ''
        The spotrak frontend package. Rebuilt against `apiEndpoint` because the
        public API origin is baked into the client bundle at build time.
      '';
    };

    user = mkOption {
      type = types.str;
      default = "spotrak";
      description = "User the spotrak services run as.";
    };

    group = mkOption {
      type = types.str;
      default = "spotrak";
      description = "Group the spotrak services run as.";
    };

    stateDir = mkOption {
      type = types.str;
      default = "/var/lib/spotrak";
      description = "Backend state directory (holds the import upload cache).";
    };

    backendPort = mkOption {
      type = types.port;
      default = 8080;
      description = "Port the backend HTTP API listens on.";
    };

    webPort = mkOption {
      type = types.port;
      default = 4322;
      description = "Port the frontend (Deno SSR) listens on.";
    };

    apiEndpoint = mkOption {
      type = types.str;
      example = "https://music.example.com";
      description = ''
        Public origin browsers reach spotrak at. Baked into the frontend bundle
        and used by the backend to build the Spotify OAuth redirect URI
        (`<apiEndpoint>/api/v1/auth/spotify/callback`, which must be registered
        in the Spotify app). The client appends `/api/v1` itself, so this must be
        the bare origin with no path.
      '';
    };

    clientEndpoint = mkOption {
      type = types.str;
      default = cfg.apiEndpoint;
      defaultText = literalExpression "config.services.spotrak.apiEndpoint";
      description = "Backend CLIENT_ENDPOINT (defaults to apiEndpoint).";
    };

    cors = mkOption {
      type = types.str;
      default = cfg.apiEndpoint;
      defaultText = literalExpression "config.services.spotrak.apiEndpoint";
      description = "Allowed CORS origin, or `*`.";
    };

    spotifyPublic = mkOption {
      type = types.str;
      description = "Spotify application client id (public).";
    };

    environmentFile = mkOption {
      type = types.either types.path types.str;
      example = "/run/secrets/spotrak.env";
      description = ''
        Path to an environment file read at service start, containing:

        ```
        SPOTIFY_SECRET=<spotify app client secret>
        SPOTIFY_TOKEN_ENCRYPTION_KEY=<openssl rand -base64 32, distinct from above>
        ```

        Provide it via your secret manager (agenix, sops-nix, ...); it must not be
        world-readable or placed in the nix store.
      '';
    };

    settings = mkOption {
      type = types.attrsOf types.str;
      default = { };
      example = {
        TIMEZONE = "America/New_York";
        LOG_LEVEL = "info";
        SPOTIFY_MARKET = "US";
      };
      description = "Extra environment variables passed to the backend.";
    };

    nginx = {
      enable = mkOption {
        type = types.bool;
        default = false;
        description = "Whether to configure an nginx virtual host that reverse-proxies spotrak.";
      };

      hostName = mkOption {
        type = types.str;
        default = builtins.replaceStrings [
          "https://"
          "http://"
        ] [
          ""
          ""
        ] cfg.apiEndpoint;
        defaultText = literalExpression ''
          builtins.replaceStrings [ "https://" "http://" ] [ "" "" ] config.services.spotrak.apiEndpoint
        '';
        example = "music.example.com";
        description = "nginx virtual host name for spotrak.";
      };
    };

    database = {
      createLocally = mkOption {
        type = types.bool;
        default = true;
        description = ''
          Provision a local PostgreSQL database and role and connect over the
          unix socket. NixOS only; on darwin set this false and supply `url`.
        '';
      };

      url = mkOption {
        type = types.str;
        default = "postgresql:///spotrak?host=/run/postgresql";
        defaultText = literalExpression ''"postgresql:///spotrak?host=/run/postgresql"'';
        description = "DATABASE_URL. Used as-is when `createLocally` is false.";
      };
    };
  };
}
