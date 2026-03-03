{ pkgs, ... }:

{
  packages = with pkgs; [
    openssl
    pkg-config
    dbus
  ];

  env.PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.dbus.dev}/lib/pkgconfig";

  enterShell = ''
    echo "🤖 Tokstat development environment"
  '';

  # See full reference at https://devenv.sh/reference/options/
}
