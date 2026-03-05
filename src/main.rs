use anyhow::{Context, Result};
use octocrab::OctocrabBuilder;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

mod analyzer;
mod changelog;
mod github;
mod utils;

fn set_output(name: &str, value: &str) -> Result<()> {
    if let Ok(output_path) = env::var("GITHUB_OUTPUT") {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(output_path)?;
        writeln!(file, "{}={}", name, value)?;
    } else {
        println!("::set-output name={}::{}", name, value);
    }
    Ok(())
}

fn set_multiline_output(name: &str, value: &str) -> Result<()> {
    if let Ok(output_path) = env::var("GITHUB_OUTPUT") {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(output_path)?;
        writeln!(file, "{}<<EOF\n{}\nEOF", name, value)?;
    } else {
        println!("::set-output name={}::\n{}", name, value);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let github_token = env::var("GITHUB_TOKEN").context("Missing GITHUB_TOKEN")?;
    let default_bump = env::var("INPUT_DEFAULT_BUMP").unwrap_or_else(|_| "patch".to_string());
    let tag_prefix = env::var("INPUT_TAG_PREFIX").unwrap_or_else(|_| "v".to_string());
    let should_fetch_all_tags =
        env::var("INPUT_FETCH_ALL_TAGS").unwrap_or_else(|_| "false".to_string()) == "true";
    let custom_tag = env::var("INPUT_CUSTOM_TAG").ok().filter(|s| !s.is_empty());
    let dry_run = env::var("INPUT_DRY_RUN").unwrap_or_else(|_| "false".to_string()) == "true";

    let generate_release =
        env::var("INPUT_GENERATE_RELEASE").unwrap_or_else(|_| "false".to_string()) == "true";
    let release_name_template =
        env::var("INPUT_RELEASE_NAME").unwrap_or_else(|_| "Release {tag}".to_string());
    let release_body_input = env::var("INPUT_RELEASE_BODY")
        .ok()
        .filter(|s| !s.is_empty());
    let draft = env::var("INPUT_DRAFT").unwrap_or_else(|_| "false".to_string()) == "true";
    let prerelease = env::var("INPUT_PRERELEASE").unwrap_or_else(|_| "false".to_string()) == "true";
    let artifacts_input = env::var("INPUT_ARTIFACTS").unwrap_or_default();

    let custom_release_rules = env::var("INPUT_CUSTOM_RELEASE_RULES").unwrap_or_default();

    let github_sha = env::var("GITHUB_SHA").context("Missing GITHUB_SHA")?;
    let github_repository = env::var("GITHUB_REPOSITORY").context("Missing GITHUB_REPOSITORY")?;
    let repo_parts: Vec<&str> = github_repository.split('/').collect();
    let owner = repo_parts[0];
    let repo = repo_parts[1];

    let octocrab = OctocrabBuilder::new()
        .personal_token(github_token.clone())
        .build()?;

    let raw_tags = github::list_tags(&octocrab, owner, repo, should_fetch_all_tags).await?;
    let valid_tags = utils::get_valid_tags(raw_tags, &tag_prefix)?;
    let latest_tag = utils::get_latest_tag(&valid_tags, &tag_prefix);

    set_output("previous_tag", &latest_tag.raw_name)?;
    set_output("previous_version", &latest_tag.version.to_string())?;

    let mut rules = changelog::get_default_rules();
    rules = changelog::merge_custom_rules(rules, &custom_release_rules);

    let new_version_str;
    let release_type;

    let mut changelog_commits = Vec::new();

    if let Some(custom) = custom_tag {
        println!("Custom tag specified. Skipping commit analysis.");
        let version_part = custom.strip_prefix(&tag_prefix).unwrap_or(&custom);
        new_version_str = version_part.to_string();
        release_type = "custom".to_string();
    } else {
        let mut commit_messages = Vec::new();

        if latest_tag.sha != "HEAD" {
            let commits =
                github::compare_commits(&octocrab, owner, repo, &latest_tag.sha, &github_sha)
                    .await?;
            println!(
                "Found {} new commits since {}.",
                commits.len(),
                latest_tag.raw_name
            );
            for commit in &commits {
                commit_messages.push(commit.commit.message.clone());
                changelog_commits.push((commit.sha.clone(), commit.commit.message.clone()));
            }
        } else {
            println!("No previous tags found. Using default bump.");
        }

        let string_slices: Vec<&str> = commit_messages.iter().map(|s| s.as_str()).collect();
        let bump_type = analyzer::analyze_commits(&string_slices, &default_bump, &rules);

        if bump_type == analyzer::BumpType::None {
            println!("No bump necessary. Exiting.");
            return Ok(());
        }

        release_type = bump_type.as_str().to_string();

        let mut new_version = latest_tag.version.clone();
        match bump_type {
            analyzer::BumpType::Major => {
                new_version.major += 1;
                new_version.minor = 0;
                new_version.patch = 0;
            }
            analyzer::BumpType::Minor => {
                new_version.minor += 1;
                new_version.patch = 0;
            }
            analyzer::BumpType::Patch => {
                new_version.patch += 1;
            }
            _ => {}
        }

        new_version_str = new_version.to_string();
    }

    let new_tag_name = format!("{}{}", tag_prefix, new_version_str);

    println!("Computed Release Type: {}", release_type);
    println!("New Version: {}", new_version_str);
    println!("New Tag: {}", new_tag_name);

    set_output("release_type", &release_type)?;
    set_output("new_version", &new_version_str)?;
    set_output("new_tag", &new_tag_name)?;

    let markdown_changelog = changelog::generate_changelog(&changelog_commits, &rules);
    println!(
        "=== Changelog ===\n{}\n=================",
        markdown_changelog
    );
    set_multiline_output("changelog", &markdown_changelog)?;

    if !dry_run {
        github::create_tag(&octocrab, owner, repo, &new_tag_name, false, &github_sha).await?;
        println!("Tag pushed successfully.");

        if generate_release {
            let release_name = release_name_template.replace("{tag}", &new_tag_name);
            let release_body = release_body_input.unwrap_or_else(|| markdown_changelog.clone());

            let release_res = github::create_release(
                &octocrab,
                owner,
                repo,
                &new_tag_name,
                &release_name,
                &release_body,
                draft,
                prerelease,
            )
            .await?;

            println!(
                "GitHub release created: {}",
                release_res["html_url"].as_str().unwrap_or("")
            );

            if let Some(upload_url) = release_res["upload_url"].as_str() {
                let patterns = artifacts_input.split(['\n', ',']);
                for pattern in patterns {
                    let pattern = pattern.trim();
                    if pattern.is_empty() {
                        continue;
                    }

                    println!("Globbing for artifacts matching: '{}'", pattern);
                    if let Ok(paths) = glob::glob(pattern) {
                        for path in paths.flatten() {
                            if path.is_file() {
                                github::upload_asset(&github_token, upload_url, &path).await?;
                            }
                        }
                    }
                }
            } else {
                println!("Warning: Release creation response did not contain an upload URL.");
            }
        }
    } else {
        println!("Dry run enabled. Skipping tag push and release creation.");
    }

    Ok(())
}
