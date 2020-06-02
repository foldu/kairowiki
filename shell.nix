let
  pkgs = import <nixpkgs> {};
in
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    openssl
    pkg-config
    sqlite
  ];

  #DB_FILE = "./data/db/db.sqlite";
  #GIT_REPO = "./data/repo";
  #DATABASE_URL = "sqlite://${DB_FILE}";

  shellHook = ''
    source .env
  '';
}
