{ pkgs }:
let
  cargoRunner =
    name:
    pkgs.writeShellApplication {
      inherit name;
      text = builtins.readFile (./. + "/${name}.bash");
    };
  push = pkgs.writeShellApplication {
    name = "push";
    runtimeInputs = [ pkgs.git ];
    text = builtins.readFile ./push.bash;
  };
  add = pkgs.writeShellApplication {
    name = "add";
    runtimeInputs = [ push ];
    text = builtins.readFile ./add.bash;
  };
in
map cargoRunner [
  "run"
  "match"
  "outlier"
  "recommend"
]
++ [
  push
  add
]
