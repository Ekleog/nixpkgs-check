{ lib, rustPlatform, pkg-config, openssl, path, makeWrapper, nix }:

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

  cargoSha256 = "11p5fghyfbbgf5gqq1r10df963pc5fv6gmw860am575cva93xrly";

  CONTRIBUTING_MD_PATH = path + "/.github/CONTRIBUTING.md";

  postInstall = ''
    wrapProgram "$out/bin/nixpkgs-check" \
      --prefix PATH : "${lib.makeBinPath [ nix ]}"
  '';
}
