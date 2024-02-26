use anyhow::{anyhow, Error as AnyError};
use regex::Regex;
use serde::Deserialize;

const SEMVER_REGEX: &str = r#"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"#;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Configuration {
    #[serde(flatten)]
    pub versions_gatherer: VersionsGatherer,
    #[serde(default = "Configuration::default_readme")]
    pub readme: bool,
    #[serde(default)]
    pub title: TitleConfiguration,
    #[serde(alias = "pattern", default = "Configuration::default_doc_pattern")]
    pub doc_pattern: String,
}

impl Configuration {
    const fn default_readme() -> bool {
        true
    }

    fn default_doc_pattern() -> String {
        "https://docs.rs/{crate}/{version}/{crate_underscore}/".to_owned()
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            versions_gatherer: VersionsGatherer::default(),
            readme: Self::default_readme(),
            title: TitleConfiguration::default(),
            doc_pattern: Self::default_doc_pattern(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TitleConfiguration {
    #[serde(default = "TitleConfiguration::default_label")]
    pub label: String,
    #[serde(default = "TitleConfiguration::default_size")]
    pub size: usize,
}

impl TitleConfiguration {
    fn default_label() -> String {
        "Versions".to_owned()
    }

    const fn default_size() -> usize {
        2
    }
}

impl Default for TitleConfiguration {
    fn default() -> Self {
        Self {
            label: Self::default_label(),
            size: Self::default_size(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum VersionsGatherer {
    #[serde(rename_all = "kebab-case")]
    Registry {
        #[serde(
            alias = "api",
            default = "VersionsGatherer::default_registry_api_base_url"
        )]
        api_base_url: String,
    },
    #[serde(rename_all = "kebab-case")]
    Git {
        #[serde(alias = "tags")]
        tags_pattern: Option<String>,
    },
}

impl VersionsGatherer {
    fn default_registry_api_base_url() -> String {
        "https://crates.io/api/v1".to_owned()
    }

    pub fn unwrap_git_tags_pattern(&self, package: &str) -> Result<Regex, AnyError> {
        match self {
            VersionsGatherer::Registry { .. } => Err(anyhow!("unexpected registry gatherer")),
            VersionsGatherer::Git { tags_pattern: None } => {
                Ok(Regex::new(&format!(r#"^{package}-v({SEMVER_REGEX})$"#))?)
            }
            VersionsGatherer::Git {
                tags_pattern: Some(regex),
            } => Ok(Regex::new(&format!(r#"^{regex}$"#))?),
        }
    }
}

impl Default for VersionsGatherer {
    fn default() -> Self {
        Self::Registry {
            api_base_url: Self::default_registry_api_base_url(),
        }
    }
}
