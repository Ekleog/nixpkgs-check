{ lib, rustPlatform, openssl, pkg-config }:

rustPlatform.buildRustPackage {
  name = "nixpkgs-check";

  src = lib.sourceFilesBySuffices ./. [".rs" ".toml" ".lock"];

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  cargoSha256 = "1gfvg393pwccb19jvc26g7qjds1k5n9v0fncs35v08abnjsg13a8";
}
