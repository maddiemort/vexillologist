# Vexillologist

Discord bot that watches for scores from the following "dle" games:

- [Flagle][flagle]
- [FoodGuessr][foodguessr]
- [Geogrid][geogrid]

Scores are stored by user and guild ID, and then both daily and all-time leaderboards are available
for each game via the `/leaderboard` slash command.

## Development

### Set up Rust toolchain

If you're using `rustup`, there's a `rust-toolchain.toml` file in the root of the repository that
specifies the toolchain needed to build the crate. To format the code, though, you'll have to
install a nightly toolchain and ensure it contains the `rustfmt` component, and then format with
`cargo +nightly fmt` or `rustup nightly run rustfmt`.

If you're using Nix, there's a flake that defines a dev shell with a Rust toolchain (including
nightly `rustfmt`) and a few other useful tools included. You can load this either with `nix
develop` or via the [`direnv`][direnv] `.envrc` file, and then build the crate as normal with `cargo
build`. Formatting should work out of the box with `cargo fmt` or `rustfmt`.

There's also a Nix package defined in the flake for release builds.

### Cutting a Release

This project uses [Conventional Commits][conventional-commits], and [`convco`][convco] is included
in the Nix devShell to assist with this.

The overall list of things that has to happen for each release is as follows:

- The commit that changes the version should use the message `release: v<version>`.
- That commit should update the version in `Cargo.toml` to match the version output by `convco
  version --bump`.
- That commit should include an updated `Cargo.lock` file, most easily generated by running a `cargo
  build` after updating the version.
- That commit should include an updated `Cargo.nix` file, generated using `cargo2nix -ls > Cargo.nix
  && nix fmt`.
- That commit should include the updated `CHANGELOG.md`, generated with `convco changelog -u
  $(convco version --bump) > CHANGELOG.md`.
  - Unfortunately, after doing this you'll have to update the file to replace `HEAD` in the URL in
    the new section heading with `v<version>`.
- That commit should be tagged with `v<version>`.
- The commit should be pushed to `main`, and the tag pushed as well. This will trigger the
  [`cargo-dist`][cargo-dist] workflow to build the artifacts and create the release on GitHub.

[cargo-dist]: https://github.com/axodotdev/cargo-dist
[convco]: https://github.com/convco/convco
[conventional-commits]: https://www.conventionalcommits.org/en/v1.0.0/
[direnv]: https://github.com/direnv/direnv
[flagle]: https://flagle.io
[foodguessr]: https://foodguessr.com
[geogrid]: https://geogridgame.com
