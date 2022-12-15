{
  config,
  pkgs,
  lib,
  ...
}:
with builtins; let
  std = pkgs.lib;
  toml = pkgs.formats.toml {};
  melia = config.services.melia;
  mdir = melia.directories;
  mproxy = melia.reverseProxy;
in {
  options.services.melia = with lib; {
    enable = mkEnableOption "melia web server";
    package = mkPackageOption "melia" {};
    user = mkOption {
      type = types.str;
      default = "melia";
    };
    group = mkOption {
      type = types.str;
      default = "melia";
    };
    directories = {
      name = mkOption {
        type = types.str;
        default = "melia";
        description = "The base name of directories used by Melia.";
      };
      runtime = mkOption {
        type = types.str;
        readOnly = true;
        default = "/run/${melia.directories.name}";
      };
      state = mkOption {
        type = types.str;
        readOnly = true;
        default = "/var/lib/${melia.directories.name}";
      };
      cache = mkOption {
        type = types.str;
        readOnly = true;
        default = "/var/cache/${melia.directories.name}";
      };
      logs = mkOption {
        type = types.str;
        readOnly = true;
        default = "/var/log/${melia.directories.name}";
      };
      configuration = mkOption {
        type = types.str;
        readOnly = true;
        default = "/etc/${melia.directories.name}";
      };
    };
    systemd = {
      unitName = mkOption {
        type = types.str;
        default = "melia";
      };
    };
    settings = mkOption {
      type = toml.type;
      default = {};
    };
    settingsFile = mkOption {
      type = types.path;
      readOnly = true;
      default = toml.generate "melia-config.toml" melia.settings;
    };
    reverseProxy = {
      type = mkOption {
        type = types.nullOr (types.enum ["nginx"]);
        default = null;
      };
      hostName = mkOption {
        type = types.str;
        default = config.networking.fqdn;
      };
      socket = mkOption {
        type = types.nullOr types.str;
        readOnly = true;
        default =
          if mproxy.type == null
          then null
          else "${mdir.runtime}/${mproxy.type}";
      };
    };
    listen = {
      addresses = mkOption {
        type = types.attrsOf (types.submoduleWith {
          modules = [
            ({
              config,
              lib,
              name,
              ...
            }: {
              options = with lib; {
                address = mkOption {
                  type = types.str;
                  readOnly = true;
                  default = name;
                };
                ports = mkOption {
                  type = types.attrsOf (types.submoduleWith {
                    modules = [
                      ({
                        config,
                        lib,
                        name,
                        ...
                      }: {
                        options = with lib; {
                          port = mkOption {
                            type = types.port;
                            readOnly = true;
                            default = name;
                          };
                          ssl = mkEnableOption "SSL";
                        };
                      })
                    ];
                  });
                  default = {
                    "80" = {};
                    "443" = {ssl = true;};
                  };
                };
              };
            })
          ];
        });
        default = [];
      };
      sockets = mkOption {
        type = types.attrsOf (types.submoduleWith {
          modules = [
            ({
              config,
              lib,
              name,
              ...
            }: {
              options = with lib; {
                path = mkOption {
                  type = types.str;
                  readOnly = true;
                  default = name;
                };
                user = mkOption {
                  type = types.str;
                  default = melia.user;
                };
                group = mkOption {
                  type = types.str;
                  default = melia.group;
                };
                mode = mkOption {
                  type = types.str;
                  default = "0660";
                };
              };
            })
          ];
        });
        default = [];
      };
    };
  };
  disabledModules = [];
  imports = [];
  config = lib.mkIf melia.enable (lib.mkMerge [
    {
      melia.settings = {
        directories = removeAttrs melia.directories ["name"];
      };
      users.users.${melia.user} = {
        isSystemUser = true;
        inherit (melia) group;
      };
      users.groups.${melia.group} = {};
      systemd.services.${melia.systemd.unitName} = {
        description = "Melia web server";
        wantedBy = ["multi-user.target"];
        after = ["network.target"];
        serviceConfig = {
          ExecStart = "${melia.package}/bin/melia -c ${melia.settingsFile}";
          User = melia.user;
          Group = melia.group;
          # files & permissions
          UMask = "0027"; # 0640 / 0750
          RuntimeDirectory = mdir.name;
          RuntimeDirectoryMode = "0755";
          StateDirectory = mdir.name;
          StateDirectoryMode = "0750";
          CacheDirectory = mdir.name;
          CacheDirectoryMode = "0750";
          LogsDirectory = mdir.name;
          LogsDirectoryMode = "0750";
          ConfigurationDirectory = mdir.name;
          ConfigurationDirectoryMode = "0750";
          # security
          NoNewPrivileges = true;
          AmbientCapabilities = ["CAP_NET_BIND_SERVICE" "CAP_SYS_RESOURCE"];
          CapabilityBoundingSet = ["CAP_NET_BIND_SERVICE" "CAP_SYS_RESOURCE"];
          ## sandboxing
          ProcSubset = "pid";
          ProtectProc = "invisible";
          ProtectSystem = "strict";
          ProtectHome = true;
          PrivateTmp = true;
          PrivateDevices = true;
          ProtectHostname = true;
          ProtectClock = true;
          ProtectKernelTunables = true;
          ProtectKernelModules = true;
          ProtectKernelLogs = true;
          ProtectControlGroups = true;
          RestrictAddressFamilies = ["AF_UNIX" "AF_INET" "AF_INET6"];
          RestrictNamespaces = true;
          LockPersonality = true;
          MemoryDenyWriteExecute = true;
          RestrictRealtime = true;
          RestrictSUIDSGID = true;
          RemoveIPC = true;
          PrivateMounts = true;
          ## syscall filtering
          SystemCallArchitectures = "native";
          SystemCallFilter = ["~@cpu-emulation @debug @keyring @mount @obsolete @privileged @setuid" "~@ipc"];
        };
      };
    }
    (lib.mkIf (mproxy.type == "nginx") {
      services.nginx.virtualHosts."${mproxy.hostName}" = {
        locations."/" = {
          proxyPass = "$scheme://unix:${mproxy.socket}:";
        };
      };
      systemd.services.${melia.systemd.unitName} = {
        after = ["nginx.service"];
      };
      services.melia.listen.sockets = {
        ${mproxy.socket} = {
          user = services.nginx.user;
          group = melia.group;
          mode = "0660";
        };
      };
    })
  ]);
  meta = {};
}
