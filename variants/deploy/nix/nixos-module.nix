# NixOS: imports = [ ./nixos-module.nix ]; services.tezaur = { enable = true; tokenFile = "/run/secrets/tezaur"; };
{ config, lib, pkgs, ... }:
let cfg = config.services.tezaur; in {
  options.services.tezaur = {
    enable = lib.mkEnableOption "TEZAUR research assistant";
    package = lib.mkOption { type = lib.types.package; default = pkgs.tezaur or (throw "set services.tezaur.package (the flake in this repository provides it)"); };
    port = lib.mkOption { type = lib.types.port; default = 8787; };
    bind = lib.mkOption { type = lib.types.str; default = "127.0.0.1"; };
    tokenFile = lib.mkOption { type = lib.types.nullOr lib.types.path; default = null; description = "File with TEZAUR_TOKEN=... (required when bind is not loopback)."; };
    modelHost = lib.mkOption { type = lib.types.str; default = "http://127.0.0.1:11434"; };
    openFirewall = lib.mkOption { type = lib.types.bool; default = false; };
  };
  config = lib.mkIf cfg.enable {
    users.users.tezaur = { isSystemUser = true; group = "tezaur"; home = "/var/lib/tezaur"; };
    users.groups.tezaur = { };
    systemd.services.tezaur = {
      wantedBy = [ "multi-user.target" ]; after = [ "network-online.target" ];
      environment = { TEZAUR_INDEX = "/var/lib/tezaur/tezaur.idx"; TEZAUR_HOST = cfg.modelHost; };
      serviceConfig = {
        User = "tezaur"; StateDirectory = "tezaur"; Restart = "on-failure";
        EnvironmentFile = lib.mkIf (cfg.tokenFile != null) cfg.tokenFile;
        ExecStart = "${cfg.package}/bin/tezaur serve --bind ${cfg.bind} --port ${toString cfg.port} --workspaces /var/lib/tezaur/workspaces";
        NoNewPrivileges = true; ProtectSystem = "strict"; ProtectHome = true; PrivateTmp = true;
      };
    };
    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];
  };
}
