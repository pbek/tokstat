(import
  (fetchTarball {
    url = "https://github.com/edolstra/flake-compat/archive/master.tar.gz";
    sha256 = "0pf91b8h4f2yck0x1jx2nzgz0zl4wkf0q8s4cxvvz1j2xvxl76cd";
  })
  {
    src = ./.;
  }
).shellNix
