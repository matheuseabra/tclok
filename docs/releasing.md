# Releasing `tclok`

## One-time setup

1. Push this repository and enable GitHub Actions.
2. Create a crates.io API token with publish scope and add it as the optional `CARGO_REGISTRY_TOKEN` repository secret. Without it, tags still create a GitHub release; publishing to crates.io is skipped.
3. Add `Formula/tclok.rb` to the public `matheuseabra/homebrew-tap` repository.

## Release

1. Update `version` in `Cargo.toml` and the Homebrew tap formula.
2. Run `sh scripts/release-check.sh <version>`.
3. Tag the release commit as `v<version>` and push the branch and tag. The release workflow runs the checks, publishes to crates.io when its token is configured, and creates the GitHub release.
4. Download the immutable GitHub tag archive and compute its checksum:

   ```sh
   curl -L -o tclok-<version>.tar.gz https://github.com/matheuseabra/tclok/archive/refs/tags/v<version>.tar.gz
   shasum -a 256 tclok-<version>.tar.gz
   ```

5. Update the `url` and `sha256` in `matheuseabra/homebrew-tap/Formula/tclok.rb`, then validate it with `brew style` and `brew install --build-from-source matheuseabra/tap/tclok`.

Users can then install it directly:

```sh
brew install matheuseabra/tap/tclok
```

The formula builds from the signed Git tag archive, so Homebrew works even before the crate is published. Once crates.io publishing is enabled, Cargo users can also run `cargo install tclok`.
