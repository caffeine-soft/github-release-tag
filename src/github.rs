use anyhow::Result;
use octocrab::Octocrab;
use octocrab::models::repos::Tag;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct CompareResult {
    pub commits: Vec<CommitItem>,
}

#[derive(Deserialize, Debug)]
pub struct CommitItem {
    pub sha: String,
    pub commit: CommitDetails,
}

#[derive(Deserialize, Debug)]
pub struct CommitDetails {
    pub message: String,
}

/// Fetch tags for a given repository recursively, matching the behavior of `listTags` in TS.
pub async fn list_tags(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    should_fetch_all_tags: bool,
) -> Result<Vec<Tag>> {
    println!("Fetching tags for {}/{}...", owner, repo);

    // Fetch the first page (up to 100 tags)
    let mut page = octocrab
        .repos(owner, repo)
        .list_tags()
        .per_page(100)
        .send()
        .await?;

    let mut tags = page.take_items();

    // If fetch_all_tags is true, continue paginating until we have all of them
    if should_fetch_all_tags && tags.len() == 100 {
        while let Ok(Some(mut next_page)) = octocrab.get_page::<Tag>(&page.next).await {
            tags.extend(next_page.take_items());
            page = next_page;
        }
    }

    Ok(tags)
}

/// Compare `head_ref` to `base_ref` (i.e. base_ref...head_ref) matching `compareCommits` in TS.
pub async fn compare_commits(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    base_ref: &str,
    head_ref: &str,
) -> Result<Vec<CommitItem>> {
    println!("Comparing commits ({}...{})", base_ref, head_ref);

    // octocrab doesn't have a strongly-typed wrapper for the compare endpoint yet,
    // so we use the raw `.get()` method with our custom Deserialize structs.
    let route = format!(
        "/repos/{}/{}/compare/{}...{}",
        owner, repo, base_ref, head_ref
    );
    let result: CompareResult = octocrab.get(route, None::<&()>).await?;

    Ok(result.commits)
}

/// Create a new tag matching `createTag` in TS.
pub async fn create_tag(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    new_tag: &str,
    create_annotated_tag: bool,
    github_sha: &str,
) -> Result<()> {
    if create_annotated_tag {
        println!("Creating annotated tag.");

        let route = format!("/repos/{}/{}/git/tags", owner, repo);
        let body = serde_json::json!({
            "tag": new_tag,
            "message": new_tag,
            "object": github_sha,
            "type": "commit"
        });

        // Create the annotated tag object
        let _tag_res: serde_json::Value = octocrab.post(&route, Some(&body)).await?;
    }

    println!("Pushing new tag to the repo.");
    let route = format!("/repos/{}/{}/git/refs", owner, repo);
    let body = serde_json::json!({
        "ref": format!("refs/tags/{}", new_tag),
        "sha": github_sha
    });

    // Create the reference (this actually makes the tag visible in the repo)
    let _ref_res: serde_json::Value = octocrab.post(&route, Some(&body)).await?;

    Ok(())
}

/// Create a new GitHub Release
#[allow(clippy::too_many_arguments)]
pub async fn create_release(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    tag_name: &str,
    name: &str,
    body: &str,
    draft: bool,
    prerelease: bool,
) -> Result<serde_json::Value> {
    println!("Creating GitHub release '{}' for tag '{}'", name, tag_name);

    let route = format!("/repos/{}/{}/releases", owner, repo);
    let payload = serde_json::json!({
        "tag_name": tag_name,
        "name": name,
        "body": body,
        "draft": draft,
        "prerelease": prerelease,
        "generate_release_notes": false
    });

    let res: serde_json::Value = octocrab.post(&route, Some(&payload)).await?;
    Ok(res)
}

/// Upload an asset to a GitHub Release
pub async fn upload_asset(
    github_token: &str,
    upload_url: &str,
    file_path: &std::path::Path,
) -> Result<()> {
    let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
    println!("Uploading asset: {}", file_name);

    let bytes = tokio::fs::read(file_path).await?;
    let mime_type = mime_guess::from_path(file_path).first_or_octet_stream();

    // The upload URL from GitHub looks like:
    // https://uploads.github.com/repos/owner/repo/releases/id/assets{?name,label}
    // We need to strip the `{?name,label}` and append `?name=filename`
    let base_url = if let Some(idx) = upload_url.find('{') {
        &upload_url[..idx]
    } else {
        upload_url
    };

    let final_url = format!("{}?name={}", base_url, file_name);

    let client = reqwest::Client::builder()
        .user_agent("github-release-tag")
        .build()?;

    let res = client
        .post(&final_url)
        .header("Authorization", format!("Bearer {}", github_token))
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Accept", "application/vnd.github+json")
        .header("Content-Type", mime_type.to_string())
        .header("Content-Length", bytes.len().to_string())
        .body(bytes)
        .send()
        .await?;

    if !res.status().is_success() {
        let text = res.text().await?;
        return Err(anyhow::anyhow!(
            "Failed to upload asset {}: {}",
            file_name,
            text
        ));
    }

    println!("Asset {} uploaded successfully.", file_name);
    Ok(())
}
