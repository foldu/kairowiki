{ openssl
, naersk
, sqlite
, pkg-config
, debug
, mime-types
}:

let
  src = builtins.filterSource
    (path: type: type != "directory" || (let basename = builtins.baseNameOf path; in basename != "target" && basename != "data"))
    ./.;
in
naersk.buildPackage {
  inherit src;
  singleStep = true;
  preBuild = ''
    $src/init_db.sh
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
    mkdir -p "$out/etc"
    cp ${mime-types}/etc/mime.types "$out/etc"
    mkdir -p "$out/usr/lib/kairowiki"
    cp -r ${src}/static "$out/usr/lib/kairowiki"
  '';
}
