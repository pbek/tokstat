_:

{

  # https://devenv.sh/languages/
  languages = {
    rust.enable = true;
  };

  packages = [
    _.nixpkgs.legacyPackages.x86_64-linux.openssl
  ];

  # https://devenv.sh/git-hooks/
  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
  };

  # See full reference at https://devenv.sh/reference/options/
}
