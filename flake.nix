{
  description = "ccwire: a Rust SDK for Claude Code";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      manifest = (nixpkgs.lib.importTOML ./crates/ccwire/Cargo.toml).package;
    in
    {
      # The whole workspace, built and tested (every feature) in the sandbox.
      # Libraries only: there is nothing to install.
      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = manifest.name;
          inherit (manifest) version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "--workspace"
            "--all-features"
          ];
          cargoTestFlags = [
            "--workspace"
            "--all-features"
          ];
          # reqwest's rustls loads the platform roots when a client is built
          # (ccwire-oauth's tests build some); the sandbox has none.
          nativeCheckInputs = [ pkgs.cacert ];
          installPhase = ''
            runHook preInstall
            mkdir -p $out
            runHook postInstall
          '';
          meta = {
            inherit (manifest) description;
            license = pkgs.lib.licenses.asl20;
          };
        };
      });

      checks = forAllSystems (pkgs: {
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            cargo-deny
          ];
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt-rfc-style);
    };
}
