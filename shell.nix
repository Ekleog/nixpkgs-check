let
  pkgs = import ./common.nix {};
in
pkgs.stdenv.mkDerivation {
  name = "nixpkgs-check-shell";
  buildInputs = (
    (with pkgs; [
      cargo
      openssl
      pkg-config
      rust-analyzer
      rustc
      rustfmt
    ])
  );
}
