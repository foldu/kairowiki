{ sources }:
let
  pkgs = import sources.nixpkgs {
    overlays = [ (import sources.nixpkgs-mozilla) ];
  };
  channel =
    pkgs.rustChannelOf {
      date = "2020-05-10";
      channel = "nightly";
    };
  rust = channel.rust;
  #channel.rust.override {
  #  extensions = [ "clippy-preview" "rustfmt-preview" "rust-src" ];
  #};
in
{
  naersk = pkgs.callPackage sources.naersk {
    rustc = rust;
    cargo = rust;
  };
  inherit rust;
}
