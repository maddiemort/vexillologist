{
  description = "A scoreboard bot for Geogrid (geogridgame.com)";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";

    cargo2nix.url = "github:cargo2nix/cargo2nix/main";
    cargo2nix.inputs.flake-utils.follows = "flake-utils";
    cargo2nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , ...
    } @ inputs:
    let
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [
          inputs.cargo2nix.overlays.default
          inputs.fenix.overlays.default

          (final: prev: {
            cargo2nix = inputs.cargo2nix.packages.${system}.default;

            rust-toolchain =
              let
                stableFor = target: target.fromToolchainFile {
                  file = ./rust-toolchain.toml;
                  sha256 = "sha256-Ngiz76YP4HTY75GGdH2P+APE/DEIx2R/Dn+BwwOyzZU=";
                };

                rustfmt = final.fenix.latest.rustfmt;
              in
              final.fenix.combine [
                rustfmt
                (stableFor final.fenix)
              ];
          })
        ];
      };

      supportedSystems = with flake-utils.lib.system; [
        aarch64-darwin
        x86_64-darwin
        x86_64-linux
      ];

      inherit (flake-utils.lib) eachSystem;
    in
    eachSystem supportedSystems (system:
    let
      pkgs = pkgsFor system;

      rustPkgs = pkgs.rustBuilder.makePackageSet {
        packageFun = import ./Cargo.nix;
        rustToolchain = pkgs.rust-toolchain;
      };

      inherit (pkgs.lib) optionals;
    in
    rec
    {
      packages = rec {
        default = vexillologist;
        vexillologist = (rustPkgs.workspace.vexillologist { }).out;
      };

      apps = rec {
        vexillologist = flake-utils.lib.mkApp {
          drv = packages.vexillologist;
        };
        default = vexillologist;
      };

      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo-shuttle
          cargo2nix
          convco
          nixpkgs-fmt
          rust-toolchain

          libiconv
        ] ++ (optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
        ]));
      };

      formatter = pkgs.nixpkgs-fmt;
    });
}
