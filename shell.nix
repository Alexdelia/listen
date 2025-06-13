{pkgs ? import <nixpkgs> {}, ...}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    openssl
    pkg-config
    rust-bin.stable.latest.default

    python3
    python3Packages.matplotlib
    ruff

    typos

    ffmpeg
    scdl
    yt-dlp
  ];

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [pkgs.openssl];

  shellHook =
    /*
    bash
    */
    ''
      git pull

      # export PATH="$HOME/.cargo/bin:$PATH"

      if [ ! -f .env ]; then
      	cp .env.example .env
      	printf "\n\n\t\033[1mplease edit the \033[35m.env\033[39m file\033[0m\n\n"
      fi

      alias run='cargo run --release'

      alias sync='git add listen.ron && git commit -m "🎶" && git push -q && cargo run --release -q'
    '';
}
