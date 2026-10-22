use cincinnati::plugins::internal::dkrv2_openshift_secondary_metadata_scraper::gpg;
use cincinnati::plugins::internal::dkrv2_openshift_secondary_metadata_scraper::plugin::DEFAULT_SIGNATURE_FETCH_TIMEOUT_SECS;

use anyhow::Result as Fallible;
use anyhow::{format_err, Context};

use futures::stream::StreamExt;
use lazy_static::lazy_static;
use reqwest::{Client, ClientBuilder};
use semver::Version;
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

use cincinnati::Release;

// OKD releases are signed by the OpenShift CI release key, and OKD clusters verify them against
// the CI signature store - see https://github.com/openshift/cluster-update-keys/blob/main/manifests/0000_90_cluster-update-keys_configmap.yaml
static SIGNATURE_BASEURL: &str =
    "https://storage.googleapis.com/openshift-ci-release/releases/signatures/openshift/release/";

lazy_static! {
  // Location of public keys, overridable with PUBKEYS_DIR to run outside the CI image
  static ref PUBKEYS_DIR: PathBuf = std::env::var_os("PUBKEYS_DIR")
      .map(PathBuf::from)
      .unwrap_or_else(|| PathBuf::from("/usr/local/share/public-keys/"));
  // base url for signature storage - see https://github.com/openshift/cluster-update-keys/blob/main/stores/store-openshift-ci-release
  static ref BASE_URL: Url = Url::parse(SIGNATURE_BASEURL).unwrap();
}

// OKD only serves 5.0 and later through Cincinnati, so older releases are not checked
const MIN_MAJOR_VERSION: u64 = 5;

// Skip releases published before OKD signatures were uploaded to the CI signature store
static SKIP_VERSIONS: &[&str] = &[
    "5.0.0-okd-scos.ec.0+amd64",
    "5.0.0-okd-scos.ec.1+amd64",
    "5.0.0-okd-scos.ec.2+amd64",
    "5.0.0-okd-scos.ec.3+amd64",
    "5.0.0-okd-scos.ec.4+multi",
    "5.0.0-okd-scos.ec.5+multi",
];

/// Extract payload value from Release if it is a Concrete release
fn payload_from_release(release: &Release) -> Fallible<String> {
    match release {
        Release::Concrete(c) => Ok(c.payload.clone()),
        _ => Err(format_err!("not a concrete release")),
    }
}

/// Generate URLs for signature store and attempt to find a valid signature
#[allow(clippy::ptr_arg)]
async fn find_signatures_for_version(
    client: &Client,
    public_keys: &gpg::Keyring,
    release: &Release,
) -> Fallible<()> {
    let payload = payload_from_release(release)?;
    let digest = payload
        .split('@')
        .last()
        .ok_or_else(|| format_err!("could not parse payload '{:?}'", payload))?;

    gpg::verify_signatures_for_digest(client, &BASE_URL, public_keys, digest).await
}

/// Iterate versions and return true if Release is included
fn is_release_in_versions(versions: &HashSet<Version>, release: &Release) -> bool {
    // Check that release version is not in skip list
    if SKIP_VERSIONS.contains(&release.version()) {
        return false;
    }
    // Strip arch identifier
    let stripped_version = release
        .version()
        .split('+')
        .next()
        .ok_or_else(|| release.version())
        .unwrap();
    let version = Version::from_str(stripped_version).unwrap();
    version.major >= MIN_MAJOR_VERSION && versions.contains(&version)
}

pub async fn run(releases: &[Release], found_versions: &HashSet<semver::Version>) -> Fallible<()> {
    println!("Checking release signatures");

    // Initialize keyring
    let public_keys = gpg::load_public_keys(&PUBKEYS_DIR)?;

    // Prepare http client
    let client: Client = ClientBuilder::new()
        .gzip(true)
        .timeout(Duration::from_secs(DEFAULT_SIGNATURE_FETCH_TIMEOUT_SECS))
        .build()
        .context("Building reqwest client")?;

    // Limit the concurrency; otherwise, we can make more requests than the system can handle
    const SIGNATURE_CHECK_CONCURRENCY: usize = 100;

    // Filter scraped images - skip CI images
    let results: Vec<Fallible<()>> = futures::stream::iter(
        releases
            .iter()
            .filter(|r| is_release_in_versions(found_versions, r)),
    )
    // Attempt to find signatures for filtered releases
    .map(|r| find_signatures_for_version(&client, &public_keys, r))
    .buffered(SIGNATURE_CHECK_CONCURRENCY)
    .collect::<Vec<Fallible<()>>>()
    .await
    // Filter to keep errors only
    .into_iter()
    .filter(|e| e.is_err())
    .collect();
    if results.is_empty() {
        Ok(())
    } else {
        Err(format_err!("Signature check errors: {:#?}", results))
    }
}
