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
  sqlx-cli,
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
    sqlx-cli
    cacert

  ];
  env.RUST_SRC_PATH = rustPlatform.rustLibSrc;
}
