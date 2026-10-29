{
  description = "TEZAUR — local, citation-grounded research assistant";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in {
      packages = forAll (pkgs: rec {
        tezaur = pkgs.rustPlatform.buildRustPackage {
          pname = "tezaur";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "--bin" "tezaur" ];
          nativeBuildInputs = [ pkgs.makeWrapper ];
          # poppler (text fallback + page images) and tesseract (OCR of scanned PDFs) on PATH
          postInstall = ''
            wrapProgram $out/bin/tezaur --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.poppler_utils pkgs.tesseract ]}
          '';
          meta = { description = "Local, citation-grounded research assistant for writing a thesis"; mainProgram = "tezaur"; };
        };
        default = tezaur;
      });
      devShells = forAll (pkgs: { default = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc pkgs.poppler_utils pkgs.tesseract pkgs.nodejs ]; }; });
    };
}
