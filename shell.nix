{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    # openssl
    # pkg-config
    rust-bin.stable.latest.default

    typos
  ];

  # shellHook = ''
  #   export PATH="$HOME/.cargo/bin:$PATH"
  # '';
}
