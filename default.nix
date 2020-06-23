{ openssl
, naersk
, sqlite
, pkg-config
, debug
, mime-types
, makeWrapper
}:

let
  src = builtins.filterSource
    (path: type: type != "directory" || (let basename = builtins.baseNameOf path; in basename != "target" && basename != "data"))
    ./.;
  mimeTypesFile = "${mime-types}/etc/mime.types";
in
naersk.buildPackage {
  inherit src;
  singleStep = true;
  preBuild = ''
    $src/init_db.sh
  '';
  DATABASE_URL = "sqlite://data/db/db.sqlite";
  MIME_TYPES_PATH = mimeTypesFile;
  release = !debug;
  doCheck = debug;
  buildInputs = [
    openssl
    sqlite
    pkg-config
    makeWrapper
  ];
  postInstall = ''
    mkdir -p "$out/usr/lib/kairowiki"
    cp -r ${src}/static "$out/usr/lib/kairowiki"
    wrapProgram "$out/bin/kairowiki" --set MIME_TYPES_PATH "${mimeTypesFile}"
  '';
}
