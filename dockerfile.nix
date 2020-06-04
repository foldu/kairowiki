let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs {};
  kairowiki = import ./release.nix;
in
pkgs.dockerTools.buildImage {
  name = "foldu/kairowiki";
  tag = "latest";
  contents = [ kairowiki ];
  config.Cmd = "/bin/kairowiki";
}
