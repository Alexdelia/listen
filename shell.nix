{pkgs ? import <nixpkgs> {}, ...}:
pkgs.mkShell {
  buildInputs = with pkgs;
    [
      git

      openssl
      pkg-config
      rust-bin.stable.latest.default

      python3
      python3Packages.matplotlib
      ruff

      typos

      mpc
      ffmpeg
      scdl
      yt-dlp
      wl-clipboard
      xdg-utils
    ]
    ++ (
      let
        run = "cargo run --release";
        push = "git add listen.ron && git commit -m \"🎶\" && git push -q && ${run} -q";
      in [
        (pkgs.writers.writeBashBin "run" {} "${run} -- $@")
        (pkgs.writers.writeBashBin "match" {} "${run} -- match $@")
        (pkgs.writers.writeBashBin "outlier" {} "${run} -- outlier $@")
        (pkgs.writers.writeBashBin "recommend" {} "${run} -- recommend $@")
        (pkgs.writers.writeBashBin "push" {} "${push}")
        (pkgs.writers.writeBashBin "add" {} "$EDITOR listen.ron && ${push}")
      ]
    );

  # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [pkgs.openssl];

  shellHook =
    /*
    bash
    */
    ''
      unset LD_LIBRARY_PATH

      git pull

      # export PATH="$HOME/.cargo/bin:$PATH"

      if [ ! -f .env ]; then
      	cp .env.example .env
      	printf "\n\n\t\033[1mplease edit the \033[35m.env\033[39m file\033[0m\n\n"
      fi

      data_dir="$PWD/target/xdg"
      completion_dir="$data_dir/bash-completion/completions"
      bin="target/release/declarative_listen"

      if [ -x "$bin" ] && [ ! "$completion_dir/declarative_listen" -nt "$bin" ]; then
      	mkdir -p "$completion_dir"
      	if "$bin" completion bash >"$completion_dir/declarative_listen" 2>/dev/null; then
      		for wrapper in run match outlier recommend; do
      			ln -sf declarative_listen "$completion_dir/$wrapper"
      		done
      	fi
      fi

      case ":$XDG_DATA_DIRS:" in
      	*":$data_dir:"*) ;;
      	*) export XDG_DATA_DIRS="$data_dir:$XDG_DATA_DIRS" ;;
      esac
    '';
}
