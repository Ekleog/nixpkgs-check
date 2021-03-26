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

  cargoSha256 = "0w4m8an5jyd3f47zrwl7fprcafdqj9x2flww0h3fvipw6mgx99ja";
}
