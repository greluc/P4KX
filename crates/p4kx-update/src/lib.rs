//! Asks the GitHub API whether a newer Release of P4KX than the running one exists.
//!
//! The request is a single anonymous GET of the latest Release; nothing about the user, the
//! machine or the Archives is sent.

use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

const GITHUB_LATEST: &str = "https://api.github.com/repos/greluc/P4KX/releases/latest";
const TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = concat!("P4KX/", env!("CARGO_PKG_VERSION"));

/// A published Release of P4KX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The version without the leading `v` of its tag, such as `0.2.0`.
    pub version: String,
    /// The web page of the Release.
    pub page: String,
}

/// Why the latest Release could not be determined.
#[derive(Debug, Error)]
pub enum UpdateError {
    /// The request failed or was answered with an error status.
    #[error("cannot ask {endpoint} for the latest Release")]
    Request {
        /// The endpoint asked.
        endpoint: String,
        /// The cause.
        #[source]
        source: Box<ureq::Error>,
    },
    /// The answer is not the JSON of a Release.
    #[error("the answer of {endpoint} is not a Release")]
    Malformed {
        /// The endpoint asked.
        endpoint: String,
        /// The cause.
        #[source]
        source: serde_json::Error,
    },
    /// A version is not a semantic version.
    #[error("{version} is not a version")]
    Version {
        /// The text that was read as a version.
        version: String,
        /// The cause.
        #[source]
        source: semver::Error,
    },
}

#[derive(Deserialize)]
struct Answer {
    tag_name: String,
    html_url: String,
}

/// The check against one releases endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseCheck {
    endpoint: String,
}

impl ReleaseCheck {
    /// Creates the check against the latest Release of this repository on GitHub.
    #[must_use]
    pub fn github() -> Self {
        Self::new(GITHUB_LATEST)
    }

    /// Creates the check against `endpoint`, which answers like the GitHub latest-Release API.
    #[must_use]
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_owned(),
        }
    }

    /// Returns the endpoint asked.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Asks for the latest Release and returns it when it is newer than `current`.
    ///
    /// # Errors
    ///
    /// [`UpdateError`] when the request fails, the answer is no Release, or either version is
    /// not a semantic version.
    pub fn newer_than(&self, current: &str) -> Result<Option<Release>, UpdateError> {
        let current = parse(current)?;
        let answer = self.latest()?;
        let version = answer
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&answer.tag_name);
        let latest = parse(version)?;
        Ok((latest > current).then(|| Release {
            version: version.to_owned(),
            page: answer.html_url,
        }))
    }

    fn latest(&self) -> Result<Answer, UpdateError> {
        let request = |source| UpdateError::Request {
            endpoint: self.endpoint.clone(),
            source: Box::new(source),
        };
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(USER_AGENT)
            .build()
            .into();
        let body = agent
            .get(&self.endpoint)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(request)?
            .body_mut()
            .read_to_string()
            .map_err(request)?;
        serde_json::from_str(&body).map_err(|source| UpdateError::Malformed {
            endpoint: self.endpoint.clone(),
            source,
        })
    }
}

fn parse(version: &str) -> Result<semver::Version, UpdateError> {
    semver::Version::parse(version).map_err(|source| UpdateError::Version {
        version: version.to_owned(),
        source,
    })
}
