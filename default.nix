{ openssl
, naersk
, sqlite
, pkg-config
, debug
}:

let
  src = builtins.filterSource
    (path: type: type != "directory" || builtins.baseNameOf path != "target")
    ./.;
in
naersk.buildPackage {
  inherit src;
  singleStep = true;
  preBuild = ''
    mkdir -p data/db
    sqlite3 data/db/db.sqlite -init ./schema.sql .exit
  '';
  DATABASE_URL = "sqlite://data/db/db.sqlite";
  release = !debug;
  doCheck = debug;
  buildInputs = [
    openssl
    sqlite
    pkg-config
  ];
}
