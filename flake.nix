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
    in {
      formatter = std.mapAttrs (system: pkgs: pkgs.default) inputs.alejandra.packages;
      packages = std.genAttrs systems (system: let
        crane = inputs.crane.lib.${system};
      in {
        default = self.packages.${system}.melia;
        melia = crane.buildPackage {
          src = crane.cleanCargoSource ./.;
        };
      });
      overlays.default = final: prev: {
        melia = self.packages.${final.crossSystem}.melia;
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
    };
}
