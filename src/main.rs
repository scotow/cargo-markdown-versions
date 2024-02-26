mod options;
mod configuration;

use std::{env, fs};
use std::path::{Path};
use anyhow::{anyhow, Error as AnyError};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use clap::Parser;
use git2::{Commit, Error, Repository, RepositoryOpenFlags};
use crate::configuration::{Configuration, VersionsGatherer};
use crate::options::Options;
use std::str;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn main() -> Result<(), AnyError> {
    let options = Options::parse();

    let mut metadata_command = MetadataCommand::new();
    metadata_command.no_deps();
    if let Some(manifest_path) = &options.manifest_path {
        metadata_command.manifest_path(manifest_path.clone());
    }
    let metadata = metadata_command.exec()?;

    let package = select_package(&metadata, options.package.as_deref(), options.manifest_path.as_deref())?.ok_or_else(|| anyhow!("failed to resolve package"))?;
    let configuration = match package.metadata.pointer("/markdown-versions") {
        Some(value) => {
            serde_json::from_value::<Configuration>(value.clone())?
        },
        None => {
            if options.default_configuration {
                Configuration::default()
            } else {
                return Err(anyhow!("missing `package.metadata.markdown-versions` field"))
            }
        }
    };

    let versions = match &configuration.versions_gatherer {
        VersionsGatherer::Registry { api_base_url } => {
            ureq::get(&format!("{}/crates/{}/versions", api_base_url.trim_end_matches('/'), package.name))
                .call()?
                .into_json::<ApiResponse>()?
                .versions
        },
        VersionsGatherer::Git { .. } => {
            let regex = configuration.versions_gatherer.unwrap_git_tags_pattern(&package.name)?;
            let repo = Repository::open(&metadata.workspace_root)?;
            let mut tags = Vec::new();
            repo.tag_foreach(|id, name| {
                if let Ok(name) = str::from_utf8(name) {
                    if let Some(version) = regex.captures(name.trim_start_matches("refs/tags/")).and_then(|captures| captures.get(1).map(|m| m.as_str().to_owned())) {
                        let commit = match repo.find_commit(id) {
                            Ok(commit) => commit,
                            Err(_) => {
                                repo.find_tag(id).unwrap().peel().unwrap().into_commit().unwrap()
                            }
                        };
                        tags.push(CrateVersion {
                            version,
                            creation: OffsetDateTime::from_unix_timestamp(commit.time().seconds()).unwrap(),
                        });
                    }
                }
                true
            })?;
            tags.sort_by(|lhs, rhs| rhs.creation.cmp(&lhs.creation));
            tags
        }
    };

    let mut readme = match (configuration.readme, package.readme()) {
        (true, Some(readme_path)) => {
            let mut readme = fs::read_to_string(readme_path)?.trim_end().to_owned();
            readme.push_str("\n\n");
            readme
        },
        _ => String::new(),
    };

    readme.push_str(&format!("{} {}\n\n", "#".repeat(configuration.title.size), configuration.title.label));
    for CrateVersion { version, creation } in versions {
        readme.push_str(&format!("- [{} - {}]({})\n", version, creation.date() ,apply_patter(&configuration.doc_pattern, &package.name, &version)));
    }

    print!("{readme}");

    Ok(())
}

fn select_package<'a>(metadata: &'a Metadata, package: Option<&str>, manifest_path: Option<&Path>) -> Result<Option<&'a Package>, AnyError> {
    if let Some(package) = package {
        return Ok(metadata.packages.iter().find(|pkg| pkg.name == package));
    }

    match metadata.packages.len() {
        0 => Ok(None),
        1 => Ok(metadata.packages.first()),
        _ => {
            let manifest_path = match manifest_path {
                Some(path) => path.to_owned(),
                None => {
                    env::current_dir()?.join("Cargo.toml")
                }
            };
            Ok(metadata.packages.iter().find(|pkg| pkg.manifest_path == manifest_path))
        }
    }
}

fn apply_patter(doc_pattern: &str, crate_name: &str, version: &str) -> String {
    doc_pattern.replace("{crate}", crate_name)
        .replace("{crate_underscore}", &crate_name.replace('-', "_"))
        .replace("{version}", version)
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    versions: Vec<CrateVersion>,
}

#[derive(Deserialize, Debug)]
struct CrateVersion {
    #[serde(rename = "num")]
    version: String,
    #[serde(rename = "created_at", deserialize_with = "CrateVersion::deserialize_datetime")]
    creation: OffsetDateTime,
}

impl CrateVersion {
    fn deserialize_datetime<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error> where D: Deserializer<'de> {
        Ok(OffsetDateTime::parse("1985-04-12T23:20:50.52Z", &Rfc3339).unwrap())
    }
}