{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      crane,
      rust-overlay,
      ...
    }:
    let
      forEachSystem =
        f:
        nixpkgs.lib.genAttrs
          [
            "aarch64-darwin"
            "x86_64-linux"
            "aarch64-linux"
            "x86_64-darwin"
          ]
          (
            system:
            f {
              pkgs = import nixpkgs {
                inherit system;
                overlays = [ rust-overlay.overlays.default ];
              };
              inherit system;
            }
          );
    in
    {
      packages = forEachSystem (
        { pkgs, system }:
        let
          toolchain = pkgs.rust-bin.stable.latest.default;
          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

          # cleanCargoSource strips non-Rust files; the bundled docs and
          # missouri fixtures must survive the filter.
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              (craneLib.filterCargoSources path type)
              || (builtins.match ".*tests/missouri/.*" path != null)
              || (builtins.match ".*\\.yml$" path != null)
              || (builtins.match ".*\\.missouri.*" path != null)
              || (builtins.match ".*/docs$" path != null)
              || (builtins.match ".*/docs/.*" path != null)
              || (builtins.match ".*/skills$" path != null)
              || (builtins.match ".*/skills/.*" path != null)
              # tests/merge_gate_guard.rs runs scripts/merge-gate.sh, so the
              # script has to reach the sandbox. Without it five of seven
              # cases fail on a missing file and the package cannot build,
              # while the gate stays green because a runner has the script.
              || (builtins.match ".*/scripts$" path != null)
              || (builtins.match ".*/scripts/.*" path != null);
          };

          commonArgs = {
            pname = "zettel";
            version = "0.1.0";
            inherit src;
            strictDeps = true;
            buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          tool = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              # Man pages and shell completions come from the built
              # binary itself, so they always match the real CLI.
              postInstall = ''
                mkdir -p $out/share/man/man1
                $out/bin/zettel gen-man $out/share/man/man1
                mkdir -p $out/share/zsh/site-functions
                mkdir -p $out/share/bash-completion/completions
                mkdir -p $out/share/fish/vendor_completions.d
                $out/bin/zettel gen-completions zsh > $out/share/zsh/site-functions/_zettel
                $out/bin/zettel gen-completions bash > $out/share/bash-completion/completions/zettel
                $out/bin/zettel gen-completions fish > $out/share/fish/vendor_completions.d/zettel.fish
              '';
              # Unit tests only in the nix check; the missouri suite runs in
              # development (it needs the missouri binary, which lives in its
              # own derivation).
              # tests/merge_gate_guard.rs drives scripts/merge-gate.sh,
              # which runs git and reads the event payload with jq. stdenv
              # builds PATH from the declared inputs, so both have to be
              # named here. A runner has them already, which is why the
              # gate stays green while the package cannot build.
              nativeCheckInputs = [ pkgs.gitMinimal pkgs.jq ];
              checkPhase = ''
                tmpHome="$(mktemp -d)"
                export HOME="$tmpHome"
                cargo test --profile release --locked
              '';
            }
          );
        in
        {
          default = tool;
          zettel = tool;
        }
      );

      devShells = forEachSystem (
        { pkgs, ... }:
        {
          default = pkgs.mkShell {
            buildInputs = [
              pkgs.rust-bin.stable.latest.default
              pkgs.jq
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];
          };
        }
      );
    };
}
