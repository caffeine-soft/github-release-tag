use anyhow::Result;
use octocrab::models::repos::Tag;
use regex::Regex;
use semver::Version;

#[derive(Debug, Clone)]
pub struct ParsedTag {
    pub raw_name: String,
    pub sha: String,
    pub version: Version,
}

/// Filters raw GitHub tags, ensuring they match the prefix and are valid SemVer.
/// Sorts them in descending order (highest version first).
pub fn get_valid_tags(tags: Vec<Tag>, tag_prefix: &str) -> Result<Vec<ParsedTag>> {
    let prefix_regex = Regex::new(&format!("^{}", regex::escape(tag_prefix)))?;

    let mut valid_tags: Vec<ParsedTag> = tags
        .into_iter()
        .filter_map(|tag| {
            // Check if tag starts with the prefix
            if !prefix_regex.is_match(&tag.name) {
                return None;
            }

            // Strip the prefix to parse the raw semver
            let version_str = prefix_regex.replace(&tag.name, "").to_string();

            // Attempt to parse as SemVer
            match Version::parse(&version_str) {
                Ok(version) => Some(ParsedTag {
                    raw_name: tag.name,
                    sha: tag.commit.sha,
                    version,
                }),
                Err(_) => {
                    println!("Found Invalid Tag: {}", tag.name);
                    None
                }
            }
        })
        .collect();

    // Sort descending (highest version first)
    valid_tags.sort_by(|a, b| b.version.cmp(&a.version));

    for tag in &valid_tags {
        println!("Found Valid Tag: {}", tag.raw_name);
    }

    Ok(valid_tags)
}

/// Finds the latest stable tag (ignoring prereleases like v1.0.0-beta).
/// Falls back to 0.0.0 if no valid stable tags exist.
pub fn get_latest_tag(valid_tags: &[ParsedTag], tag_prefix: &str) -> ParsedTag {
    valid_tags
        .iter()
        .find(|tag| tag.version.pre.is_empty()) // .pre is empty for stable releases
        .cloned()
        .unwrap_or_else(|| {
            println!("No stable tags found. Defaulting to 0.0.0");
            ParsedTag {
                raw_name: format!("{}0.0.0", tag_prefix),
                sha: "HEAD".to_string(),
                version: Version::new(0, 0, 0),
            }
        })
}
