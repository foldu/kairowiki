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
    sqlite3 data/db/db.sqlite -init ./sql/migrations_schema.sql .exit
    sqlite3 data/db/db.sqlite -init ./sql/user_schema.sql .exit
  '';
  DATABASE_URL = "sqlite://data/db/db.sqlite";
  release = !debug;
  doCheck = debug;
  buildInputs = [
    openssl
    sqlite
    pkg-config
  ];
  postInstall = ''
    # FIXME: should copy into more UNIXy path like /usr/lib/kairowiki
    mkdir -p "$out/static"
    cp -r ${src}/static "$out"
  '';
}
