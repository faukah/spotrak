{
  mkShell,
  pkg-config,
  rustc,
  clippy,
  rust-analyzer-unwrapped,
  tombi,
  rustPlatform,
  cargo,
  rustfmt,
  cacert,
  deno,
  oxlint,
}:
mkShell {
  packages = [
    pkg-config
    clippy
    rustc
    rust-analyzer-unwrapped
    rustfmt
    tombi
    cargo

    deno
    oxlint
    cacert

  ];
  env.RUST_SRC_PATH = rustPlatform.rustLibSrc;
}
