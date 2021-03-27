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

  cargoSha256 = "0k7m0i7lxzl5hzjlc8jm9r4yh8z6lypynz9xhdjpcyyk8911jmlw";
}
