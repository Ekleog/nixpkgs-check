let
  pkgs = import ./common.nix {};
in
pkgs.stdenv.mkDerivation {
  name = "nixpkgs-check-shell";
  buildInputs = (
    (with pkgs; [
      cargo
      rust-analyzer
      rustc
      rustfmt
    ])
  );
}
