{
  description = "tokstat - Monitor token quotas across multiple AI providers";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        # Read version from Cargo.toml
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

      in
      {
        packages = {
          default = pkgs.rustPlatform.buildRustPackage rec {
            pname = "tokstat";
            inherit (cargoToml.package) version;

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
              installShellFiles
            ];

            buildInputs = with pkgs; [
              openssl
              dbus
            ];

            # For keyring support
            PKG_CONFIG_PATH = "${pkgs.dbus.dev}/lib/pkgconfig";

            postInstall = ''
              # Generate shell completions
              installShellCompletion --cmd tokstat \
                --bash <($out/bin/tokstat --generate bash 2>/dev/null) \
                --fish <($out/bin/tokstat --generate fish 2>/dev/null) \
                --zsh <($out/bin/tokstat --generate zsh 2>/dev/null)
            '';

            meta = with pkgs.lib; {
              description = "A beautiful CLI application to monitor token quotas across multiple AI providers";
              homepage = "https://github.com/pbek/tokstat";
              changelog = "https://github.com/pbek/tokstat/releases/tag/v${version}";
              license = licenses.gpl3Plus;
              maintainers = with maintainers; [ pbek ];
              mainProgram = "tokstat";
            };
          };

          # Docker image (optional)
          docker = pkgs.dockerTools.buildLayeredImage {
            name = "tokstat";
            tag = "latest";
            contents = [ self.packages.${system}.default ];
            config = {
              Cmd = [ "/bin/tokstat" ];
              Env = [
                "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              ];
            };
          };
        };

        # NixOS module (optional, for system-wide installation)
        nixosModules.default =
          { config, lib, ... }:
          with lib;
          let
            cfg = config.programs.tokstat;
          in
          {
            options.programs.tokstat = {
              enable = mkEnableOption "tokstat";

              package = mkOption {
                type = types.package;
                inherit (self.packages.${system}) default;
                description = "The tokstat package to use";
              };
            };

            config = mkIf cfg.enable {
              environment.systemPackages = [ cfg.package ];
            };
          };

        # Home Manager module (optional)
        homeManagerModules.default =
          { config, lib, ... }:
          with lib;
          let
            cfg = config.programs.tokstat;
          in
          {
            options.programs.tokstat = {
              enable = mkEnableOption "tokstat";

              package = mkOption {
                type = types.package;
                inherit (self.packages.${system}) default;
                description = "The tokstat package to use";
              };
            };

            config = mkIf cfg.enable {
              home.packages = [ cfg.package ];
            };
          };

        # Format the flake
        formatter = pkgs.nixpkgs-fmt;

        # Apps for easy running
        apps = {
          default = {
            type = "app";
            program = "${self.packages.${system}.default}/bin/tokstat";
          };
        };
      }
    );
}
