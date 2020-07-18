{ openssl
, naersk
, sqlite
, pkg-config
, debug
, mime-types
, makeWrapper
, callPackage
, lld_10
, perl
}:

let
  src = builtins.filterSource
    (
      path: type: type != "directory" || (
        let
          basename = builtins.baseNameOf path;
        in
          basename != "target" && basename != "data" && basename != "web"
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
  RUSTFLAGS = "-C link-arg=-fuse-ld=lld";
  release = !debug;
  doCheck = debug;
  nativeBuildInputs = [
    openssl
    sqlite
    pkg-config
    makeWrapper
    lld_10
    perl
  ];
  postInstall = ''
    mkdir -p "$out/usr/lib/kairowiki/static"
    cp -r ${web}/dist/* "$out/usr/lib/kairowiki/static"
    wrapProgram "$out/bin/kairowiki" --set MIME_TYPES_PATH "${mimeTypesFile}"
  '';
}
