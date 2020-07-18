let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs {};
  rust = import ./nix/rust.nix {
    inherit sources;
  };
in
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    pkg-config
    openssl
    sqlite
    yarn
  ];
  MIME_TYPES_PATH = "${pkgs.mime-types}/etc/mime.types";
  RUSTFLAGS = "-C link-arg=-fuse-ld=lld";
}
