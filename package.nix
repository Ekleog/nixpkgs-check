{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  name = "nixpkgs-check";

  src = lib.sourceFilesBySuffices ./. [".rs" ".toml" ".lock"];

  cargoSha256 = "1cj30xd6fwm0bw3saj218w5ncsij9nh9kaqcls3x4sfd9745cgn4";
}
