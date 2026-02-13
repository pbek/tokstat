{
  description = "tokstat - Monitor token quotas across multiple AI providers";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        # Separate pkgs instances: one with overlay for building, one without for outputs
        pkgsWithOverlay = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        pkgs = import nixpkgs {
          inherit system;
        };

        rustToolchain = pkgsWithOverlay.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        # Native build inputs for different platforms
        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          installShellFiles
        ];

        buildInputs =
          with pkgs;
          [
            openssl
            dbus
          ]
          ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

      in
      {
        packages = {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "tokstat";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            inherit nativeBuildInputs buildInputs;

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
