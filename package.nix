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

  cargoSha256 = "0n75n2xc29paxa6c0lpk642gxjzj1nxy0bd68jdndldbd4i8miwr";
}
