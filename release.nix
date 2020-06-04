{ debug ? false }:
let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs {};
  rust = import ./nix/rust.nix {
    inherit sources;
  };
in
pkgs.callPackage ./default.nix {
  inherit (rust) naersk;
  debug = debug;
}
