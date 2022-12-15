{
  description = "A static site generator designed for ashwalker.net";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    alejandra = {
      url = "github:kamadorueda/alejandra";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs @ {
    self,
    nixpkgs,
    crane,
    ...
  }:
    with builtins; let
      std = nixpkgs.lib;
      systems = attrNames crane.lib; # systems supported by crane
      nixpkgsFor = std.genAttrs systems (system:
        import nixpkgs {
          localSystem = builtins.currentSystem or system;
          crossSystem = system;
          overlays = [self.overlays.default];
        });
    in {
      formatter = std.mapAttrs (system: pkgs: pkgs.default) inputs.alejandra.packages;
      packages = std.genAttrs systems (system: let
        crane = inputs.crane.lib.${system};
        pkgs = nixpkgsFor.${system};
      in {
        default = self.packages.${system}.melia;
        melia = crane.buildPackage {
          src = crane.cleanCargoSource ./.;
          meta = {
            homepage = "https://github.com/SignalWalker/melia";
            description = "A static site generator designed for ashwalker.net";
            license = [std.licenses.agpl3Plus];
          };
        };
      });
      overlays.default = final: prev: {
        melia = self.packages.${final.system}.melia;
      };
      apps =
        std.mapAttrs (system: pkgs: {
          melia = {
            type = "app";
            program = "${pkgs.melia}/bin/melia";
          };
          default = self.apps.${system}.melia;
        })
        self.packages;
      nixosModules.default = import nixos-module.nix;
      devShells =
        std.mapAttrs (system: selfPkgs: let
          pkgs = nixpkgsFor.${system};
          shellPkgs = with pkgs; [just nushell cargo-watch python3 systemfd];
          shellHook = ''
            export RUST_BACKTRACE=1
            export MELIA_LOG_FILTER="warn,melia=debug"
          '';
        in {
          melia = pkgs.mkShell {
            packages = shellPkgs;
            inputsFrom = with pkgs; [melia];
            inherit shellHook;
          };
          melia-no-inputs = pkgs.mkShell {
            packages = shellPkgs;
            inputsFrom = with pkgs; [];
            inherit shellHook;
          };
          default = self.devShells.${system}.melia;
        })
        self.packages;
    };
}
