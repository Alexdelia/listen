{
  description = "listen flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rust = import ./dx/rust { inherit pkgs; };
      command = import ./dx/command { inherit pkgs; };
    in
    {
      devShells.${system} = import ./dx/shell { inherit pkgs rust command; };
    };
}
