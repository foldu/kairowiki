let
  pkgs = import <nixpkgs> {};
in
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    openssl
    pkg-config
  ];

  DB_FILE = "./data/db/db.sqlite";
  GIT_REPO = "./data/repo";
  DATABASE_URL = "sqlite://${DB_FILE}";

  shellHook = ''
    mkdir -p "$(dirname "${DB_FILE}")"
    if ! test -f "${DB_FILE}"; then
      sqlite3 "${DB_FILE}" -init ./schema.sql .exit
    fi
  '';
}
