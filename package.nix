{ lib, rustPlatform, openssl, pkg-config, makeWrapper, nix }:

rustPlatform.buildRustPackage {
  name = "nixpkgs-check";

  src = lib.sourceFilesBySuffices ./. [".rs" ".toml" ".lock"];

  nativeBuildInputs = [
    makeWrapper
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  cargoSha256 = "1axvlqwvy9br8i4v45swhvdkq0804qrafdrbhay7s1bq2sm1q8ml";

  postInstall = ''
    wrapProgram "$out/bin/nixpkgs-check" \
      --prefix PATH : "${lib.makeBinPath [ nix ]}"
  '';
}
