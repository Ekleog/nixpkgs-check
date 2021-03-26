{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  name = "nixpkgs-check";

  src = lib.sourceFilesBySuffices ./. [".rs" ".toml" ".lock"];

  cargoSha256 = "1iwk1pz2xn165frg0wr6560in0im2aka6f22hrq6ajmrc0hb393g";
}
