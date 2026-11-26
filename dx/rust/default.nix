{ pkgs }:
{
  toolchain = pkgs.rust-bin.fromRustupToolchainFile ../../rust-toolchain.toml;
  env = {
    DUCKDB_LIB_DIR = "${pkgs.lib.getLib pkgs.duckdb}/lib";
    DUCKDB_INCLUDE_DIR = "${pkgs.lib.getDev pkgs.duckdb}/include";
  };
  buildInputs = with pkgs; [
    openssl
    duckdb
  ];
  nativeBuildInputs = [ pkgs.pkg-config ];
}
