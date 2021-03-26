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

  cargoSha256 = "0prw5ipg2z8gafs2y6xwszrjgab8l5m0nms5sw7w6f9mz8mdd6wc";
}
