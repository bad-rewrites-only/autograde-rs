{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem
    (system: let
      pkgs = import nixpkgs {inherit system;};
    in
      with pkgs; rec {
        devShell = mkShell rec {
          packages = [
            digital
          ];
          buildInputs = [
          ];
          DIGITAL_JAR = "${pkgs.digital}/share/java/Digital.jar";
          # RUST_LOG = "info";
          RUST_LOG = "autograde=debug";
        };
      });
}
