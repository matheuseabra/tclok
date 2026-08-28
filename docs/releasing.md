# Releasing `tclok`

## One-time setup

1. Create the public GitHub repository at `matheuseabra/tclok` (or update the URLs in `Cargo.toml`, `README.md`, and `Formula/tclok.rb`).
2. Push this repository and enable GitHub Actions.
3. Create a crates.io API token with publish scope, then add it as the `CARGO_REGISTRY_TOKEN` repository secret.

## Release

1. Update `version` in `Cargo.toml` and `Formula/tclok.rb`.
2. Run `sh scripts/release-check.sh <version>`.
3. Compute the package checksum:

   ```sh
   shasum -a 256 target/package/tclok-<version>.crate
   ```

4. Copy that value into `Formula/tclok.rb`, commit, and tag the same commit as `v<version>`.
5. Push the branch and tag. The release workflow runs the checks, publishes to crates.io, and creates the GitHub release.

After GitHub has the `Formula/tclok.rb` file, users can install it directly:

```sh
brew install matheuseabra/tclok/tclok
```

The formula intentionally builds from the published crates.io source, so its checksum and Cargo package contents are both release inputs.
