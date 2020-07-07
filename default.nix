{ openssl
, naersk
, sqlite
, pkg-config
, debug
, mime-types
, makeWrapper
, callPackage
}:

let
  src = builtins.filterSource
    (
      path: type: type != "directory" || (
        let
          basename = builtins.baseNameOf path;
        in
          basename != "target" && basename != "data" && basename != web
      )
    )
    ./.;
  mimeTypesFile = "${mime-types}/etc/mime.types";
  web = callPackage ./web { inherit debug; };
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
    mkdir -p "$out/usr/lib/kairowiki/static"
    cp -r ${web}/dist/* "$out/usr/lib/kairowiki/static"
    wrapProgram "$out/bin/kairowiki" --set MIME_TYPES_PATH "${mimeTypesFile}"
  '';
}
