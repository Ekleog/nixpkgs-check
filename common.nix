{}:

let
  pkgsSrc = builtins.fetchTarball {
    # The following is for nixos-unstable on 2021-03-20
    url = "https://github.com/NixOS/nixpkgs/archive/bc202733924c2b0f1a338f853de08c6a0459ba54.tar.gz";
    sha256 = "0sfwng9gw97arvsn7dpdcawkn2mvnr2skhw6ppd3dkhjw2l2pavk";
  };
  pkgs = import pkgsSrc {
    overlays = [
      (self: super: {
        nixpkgs-check = self.callPackage ./package.nix {};
      })
    ];
  };
in
  pkgs
