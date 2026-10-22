# graph-data.rs
This directory is a Rust project that generates binaries that are executed in CI testing to ensure that the data in this repo can be consumed by cincinnati.

It is ported from [openshift/cincinnati-graph-data](https://github.com/openshift/cincinnati-graph-data/tree/master/graph-data.rs) and adapted for OKD:

* `verify_yaml` checks `channels/` and `blocked-edges/` (OKD has no `internal-channels/`).
* `check_releases` checks that every referenced version is published to `quay.io/okd/scos-release`, the repository the OKD Cincinnati graph-builder scrapes.
* `check_signatures` verifies release signatures in the [OpenShift CI signature store](https://github.com/openshift/cluster-update-keys/blob/main/stores/store-openshift-ci-release) against the OpenShift CI key in [public-keys](public-keys), which is what OKD clusters trust.
  Only 5.0 and later releases are checked, since OKD doesn't serve older versions through Cincinnati.
  Releases published before OKD signatures were uploaded to that store are listed in `SKIP_VERSIONS`.

OpenShift's `check_channels` (every `stable` release is also in `fast` and `candidate`) and `check_errata_link` are not ported: OKD has no `fast` channel, its `candidate` channels carry only engineering candidates, and OKD publishes no errata.

To run it locally from the repository root:

```bash
cargo build --manifest-path graph-data.rs/Cargo.toml
PUBKEYS_DIR=graph-data.rs/public-keys graph-data.rs/target/debug/cincinnati-graph-data
```

The key in `public-keys` comes from [openshift/cluster-update-keys](https://github.com/openshift/cluster-update-keys/tree/main/keys).
Only the current signing key is kept; the older `openshift-ci`, `-2` and `-3` keys have expired.
`verifier-public-key-openshift-ci-4` is a minimal export (`gpg --export-options export-minimal`) because its cross-certification from the `openshift-ci-3` key fails to load in cincinnati's keyring parser.
