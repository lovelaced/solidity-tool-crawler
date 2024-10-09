mod downloader;
mod parser;
mod filter;
mod github_api;

use chrono::{Utc, Duration};
use dotenv::dotenv;
use std::env;
use std::collections::HashSet;
use std::fs;
use filter::filter_solidity_repos;
use parser::parse_push_events;
use futures::future::join_all;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Load environment variables from .env file
    dotenv().ok();

    let end_date = Utc::now();
    let start_date = end_date - Duration::days(1);

    // Download GH Archive data for the past x days
    downloader::download_gharchive_data(start_date, end_date).await?;

    // Read and process all downloaded files
    let files = fs::read_dir("gharchive_data")?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().display().to_string())
        .collect::<Vec<_>>();

    // Retrieve GitHub token from environment
    let github_token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN must be set in .env");

    // First phase: Process all files in parallel, gathering repos that need API checks
    let (repos_to_check, hardhat_repos, foundry_repos) = process_files_in_parallel(files).await?;

    // Second phase: Run the GitHub API checks for flagged repos, tracking progress
    let (final_hardhat_repos, final_foundry_repos) = run_github_api_checks(repos_to_check, &github_token, hardhat_repos, foundry_repos).await?;

    // Final results output
    println!("Final Hardhat Repos: {:?}", final_hardhat_repos);
    println!("Final Foundry Repos: {:?}", final_foundry_repos);

    Ok(())
}

// First phase: Process files and collect repos to check
async fn process_files_in_parallel(
    files: Vec<String>,
) -> Result<(HashSet<String>, HashSet<String>, HashSet<String>), Box<dyn Error + Send + Sync>> {
    let mut tasks = Vec::new();

    // Iterate over the files and spawn async tasks using tokio::spawn
    for file in files.iter() {
        let file = file.clone(); // Clone the file to avoid lifetime issues
        tasks.push(tokio::spawn(async move {
            let push_events = parse_push_events(&file);
            filter_solidity_repos(push_events).await
        }));
    }

    // Wait for all tasks to complete
    let results = join_all(tasks).await;

    let mut repos_to_check = HashSet::new();
    let mut hardhat_repos = HashSet::new();
    let mut foundry_repos = HashSet::new();

    // Collect the results from all tasks
    for result in results {
        if let Ok((repos, hh_repos, fd_repos)) = result {
            repos_to_check.extend(repos);
            hardhat_repos.extend(hh_repos);
            foundry_repos.extend(fd_repos);
        }
    }

    Ok((repos_to_check, hardhat_repos, foundry_repos))
}

// Second phase: Run GitHub API checks
async fn run_github_api_checks(
    repos_to_check: HashSet<String>,
    token: &str,
    mut hardhat_repos: HashSet<String>,
    mut foundry_repos: HashSet<String>,
) -> Result<(HashSet<String>, HashSet<String>), Box<dyn Error + Send + Sync>> {

    // Get the total number of repos to process
    let total_repos = repos_to_check.len();
    println!("Total repos to process: {}", total_repos);

    // Set a counter to track progress
    let mut processed_repos = 0;
    let progress_interval = 200; // Print progress every 200 repos

    // Process each repo for GitHub API checks
    for repo_name in repos_to_check {
        let mut found_anything = false;

        if !hardhat_repos.contains(&repo_name) {
            if github_api::check_repo_for_file(&repo_name, "hardhat.config.js", token).await
                || github_api::check_repo_for_file(&repo_name, "hardhat.config.ts", token).await
            {
                hardhat_repos.insert(repo_name.clone());
                println!("Hardhat config found via API in repo {}", repo_name);
                found_anything = true;
            }
        }

        if !foundry_repos.contains(&repo_name) {
            if github_api::check_repo_for_file(&repo_name, "foundry.toml", token).await {
                foundry_repos.insert(repo_name.clone());
                println!("Foundry config found via API in repo {}", repo_name);
                found_anything = true;
            }
        }

        // Print "Nothing found" message if no relevant files were found
        if !found_anything {
            println!("Nothing found in {}", repo_name);
        }

        // Increment processed count
        processed_repos += 1;

        // Print progress periodically every 200 repos
        if processed_repos % progress_interval == 0 {
            println!(
                "Processed {}/{} repos... (Hardhat Repos: {}, Foundry Repos: {})",
                processed_repos,
                total_repos,
                hardhat_repos.len(),
                foundry_repos.len()
            );
        }
    }

    // Final progress output
    println!(
        "Finished processing all {} repos. Total Hardhat Repos: {}, Total Foundry Repos: {}",
        total_repos,
        hardhat_repos.len(),
        foundry_repos.len()
    );

    Ok((hardhat_repos, foundry_repos))
}

