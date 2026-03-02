use crate::changelog::ReleaseRule;
use git_conventional::Commit;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BumpType {
    None,
    Patch,
    Minor,
    Major,
}

impl BumpType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BumpType::Major => "major",
            BumpType::Minor => "minor",
            BumpType::Patch => "patch",
            BumpType::None => "none",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "major" => BumpType::Major,
            "minor" => BumpType::Minor,
            "patch" => BumpType::Patch,
            _ => BumpType::None,
        }
    }
}

/// Analyzes commit messages to determine the appropriate SemVer bump.
#[allow(clippy::collapsible_if)]
pub fn analyze_commits(
    commit_messages: &[&str],
    default_bump: &str,
    rules: &HashMap<String, ReleaseRule>,
) -> BumpType {
    let mut highest_bump = BumpType::None;

    for message in commit_messages {
        // Attempt to parse the commit message
        if let Ok(commit) = Commit::parse(message) {
            // Any breaking change immediately forces a Major bump
            if commit.breaking() {
                return BumpType::Major;
            }

            let commit_type = commit.type_().as_str().to_lowercase();
            if let Some(rule) = rules.get(&commit_type) {
                if highest_bump < rule.bump {
                    highest_bump = rule.bump.clone();
                }
            }
        }
    }

    // Fallback to the default_bump input if no conventional commits triggered a bump
    if highest_bump == BumpType::None {
        println!(
            "No conventional commit keywords found. Using default bump: {}",
            default_bump
        );
        match default_bump.to_lowercase().as_str() {
            "major" => BumpType::Major,
            "minor" => BumpType::Minor,
            "patch" => BumpType::Patch,
            "false" => BumpType::None,
            _ => BumpType::Patch, // Absolute fallback
        }
    } else {
        highest_bump
    }
}
