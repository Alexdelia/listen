{
  pkgs,
  rust,
  command,
}:
{
  default = pkgs.mkShell (
    rust.env
    // {
      packages =
        with pkgs;
        [
          git

          python3
          python3Packages.matplotlib
          ruff

          typos

          mpc
          ffmpeg
          rsgain
          scdl
          yt-dlp
          wl-clipboard
          xdg-utils

          uutils-coreutils-noprefix
          rsync
          gnutar
          bzip2
        ]
        ++ [ rust.toolchain ]
        ++ rust.nativeBuildInputs
        ++ command;
      inherit (rust) buildInputs;
      shellHook = builtins.readFile ./hook.bash;
    }
  );
}
