{
  cudaCapability ? [ "7.5" ],
  pkgs ? import (builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/151fa4e8ddfdd8dd25d945ad94ed54a13de9f6e4.tar.gz";
    sha256 = "0rm5v6n0kgqq5xrc34imn8nxx0y2xlmhpa0nag9sk6m6ys9slaij";
  }) {
    config = {
      allowUnfree = true;
      cudaCapabilities = cudaCapability;
    };
  },
}:
let
  python = pkgs.python313.override {
    packageOverrides = final: prev: {
      torch-bin = prev.torch-bin.override { cudaPackages = pkgs.cudaPackages_13; };
    };
  };
  env = python.withPackages (p: [
    p.torch-bin
    p.transformers
    p.numpy
  ]);
in
pkgs.writeShellApplication {
  name = "maest";
  runtimeInputs = [
    env
    pkgs.ffmpeg
  ];
  text = ''
    unset PYTHONPATH PYTHONHOME
    export LD_LIBRARY_PATH=/run/opengl-driver/lib
    export HF_HUB_OFFLINE=1 TRANSFORMERS_VERBOSITY=error PYTHONWARNINGS=ignore
    exec python -I ${./main.py} "$@"
  '';
}
