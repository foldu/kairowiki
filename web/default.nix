{ mkYarnPackage
, debug
}:
mkYarnPackage rec {
  name = "kairowiki-web";
  src = builtins.filterSource
    (
      path: type: type != "directory" || (
        let
          basename =
            builtins.baseNameOf path;
        in
          basename != "node_modules" && basename != "dist"
      )
    )
    ./.;
  distPhase = ''
    cd "$out/libexec/${name}/deps/${name}"
    mkdir -p "$out/dist"
    yarn run --offline webpack ${if debug then "" else "-p"} --output-path "$out/dist"
  '';
}
