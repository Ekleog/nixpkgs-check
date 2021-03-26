{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  name = "nixpkgs-check";

  src = lib.sourceFilesBySuffices ./. [".rs" ".toml" ".lock"];

  cargoSha256 = "0j6l3d2glx0rjnn5a8l702nfjm8rjfx21dca01fn2jhl630xdma8";
}
