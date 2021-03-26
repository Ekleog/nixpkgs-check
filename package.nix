{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  name = "nixpkgs-check";

  src = lib.sourceFilesBySuffices ./. [".rs" ".toml" ".lock"];

  cargoSha256 = "0rmgih407kw5sahhpr46i160idrhqamrwfcyn2568f47qxmzb1b2";
}
