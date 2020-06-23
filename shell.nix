let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs {};
  rust = import ./nix/rust.nix {
    inherit sources;
  };
in
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    openssl
    pkg-config
    sqlite
  ];
  MIME_TYPES_PATH = "${pkgs.mime-types}/etc/mime.types";
}
