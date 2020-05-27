{ openssl
, naersk
, sqlite
}:

let
  src = builtins.filterSource
    (path: type: type != "directory" || builtins.baseNameOf path != "target")
    ./.;
in
naersk.buildPackage {
  inherit src;
  singleStep = true;
  buildInputs = [
    openssl
    sqlite
  ];
}
