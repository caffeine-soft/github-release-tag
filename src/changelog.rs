use crate::analyzer::BumpType;
use git_conventional::Commit;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ReleaseRule {
    pub bump: BumpType,
    pub section: Option<String>,
}

/// Returns the default Angular conventions used by the original action.
pub fn get_default_rules() -> HashMap<String, ReleaseRule> {
    let mut rules = HashMap::new();

    // Default Semantic Versioning bumps
    rules.insert(
        "feat".to_string(),
        ReleaseRule {
            bump: BumpType::Minor,
            section: Some("Features".to_string()),
        },
    );
    rules.insert(
        "fix".to_string(),
        ReleaseRule {
            bump: BumpType::Patch,
            section: Some("Bug Fixes".to_string()),
        },
    );

    // Default Changelog sections (No version bump)
    rules.insert(
        "perf".to_string(),
        ReleaseRule {
            bump: BumpType::None,
            section: Some("Performance Improvements".to_string()),
        },
    );
    rules.insert(
        "revert".to_string(),
        ReleaseRule {
            bump: BumpType::None,
            section: Some("Reverts".to_string()),
        },
    );
    rules.insert(
        "docs".to_string(),
        ReleaseRule {
            bump: BumpType::None,
            section: Some("Documentation".to_string()),
        },
    );
    rules.insert(
        "style".to_string(),
        ReleaseRule {
            bump: BumpType::None,
            section: Some("Styles".to_string()),
        },
    );
    rules.insert(
        "refactor".to_string(),
        ReleaseRule {
            bump: BumpType::None,
            section: Some("Code Refactoring".to_string()),
        },
    );
    rules.insert(
        "test".to_string(),
        ReleaseRule {
            bump: BumpType::None,
            section: Some("Tests".to_string()),
        },
    );
    rules.insert(
        "build".to_string(),
        ReleaseRule {
            bump: BumpType::None,
            section: Some("Build Systems".to_string()),
        },
    );
    rules.insert(
        "ci".to_string(),
        ReleaseRule {
            bump: BumpType::None,
            section: Some("Continuous Integration".to_string()),
        },
    );

    rules
}

/// Parses the `custom_release_rules` input. Format: `<keyword>:<release_type>:<changelog_section>`
pub fn merge_custom_rules(
    mut base_rules: HashMap<String, ReleaseRule>,
    custom_rules_str: &str,
) -> HashMap<String, ReleaseRule> {
    if custom_rules_str.is_empty() {
        return base_rules;
    }

    for rule in custom_rules_str.split(',') {
        let parts: Vec<&str> = rule.split(':').collect();
        if parts.len() >= 2 {
            let keyword = parts[0].trim().to_lowercase();
            let bump = BumpType::from_str(parts[1].trim());

            // If section is provided, use it. Otherwise, keep the default section if it existed.
            let section = if parts.len() >= 3 && !parts[2].trim().is_empty() {
                Some(parts[2].trim().to_string())
            } else {
                base_rules.get(&keyword).and_then(|r| r.section.clone())
            };

            base_rules.insert(keyword, ReleaseRule { bump, section });
        }
    }
    base_rules
}

/// Generates the Markdown changelog grouped by sections.
#[allow(clippy::collapsible_if)]
pub fn generate_changelog(
    commits: &[(String, String)], // Tuple of (SHA, Message)
    rules: &HashMap<String, ReleaseRule>,
) -> String {
    // Group commits by section name: HashMap<SectionName, Vec<MarkdownItem>>
    let mut grouped_commits: HashMap<String, Vec<String>> = HashMap::new();

    for (sha, message) in commits {
        let parsed = Commit::parse(message.trim());

        if let Ok(commit) = parsed {
            let commit_type = commit.type_().as_str().to_lowercase();

            // Check if this commit type has a section defined in our rules
            if let Some(rule) = rules.get(&commit_type) {
                if let Some(section_name) = &rule.section {
                    let short_sha = sha.chars().take(7).collect::<String>();
                    let desc = commit.description();
                    let markdown_item = format!("* {} ({})", desc, short_sha);

                    grouped_commits
                        .entry(section_name.clone())
                        .or_default()
                        .push(markdown_item);
                }
            }
        }
    }

    // Build the final Markdown string
    let mut changelog = String::new();
    let mut sections: Vec<&String> = grouped_commits.keys().collect();
    sections.sort(); // Sort alphabetically for consistent output

    for section in sections {
        changelog.push_str(&format!("### {}\n\n", section));
        if let Some(items) = grouped_commits.get(section) {
            for item in items {
                changelog.push_str(&format!("{}\n", item));
            }
        }
        changelog.push('\n');
    }

    changelog
}
