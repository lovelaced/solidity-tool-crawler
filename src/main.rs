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

    // Process all files in parallel, ensuring unique repos
    match process_files_in_parallel(files, &github_token).await {
        Ok((hardhat_repos, foundry_repos)) => {
            println!("Hardhat Repos: {:?}", hardhat_repos);
            println!("Foundry Repos: {:?}", foundry_repos);
        },
        Err(e) => {
            eprintln!("Error occurred while processing: {}", e);
        }
    }

    Ok(())
}

async fn process_files_in_parallel(
    files: Vec<String>,
    token: &str,
) -> Result<(HashSet<String>, HashSet<String>), Box<dyn Error + Send + Sync>> {
    let mut tasks = Vec::new();

    // Iterate over the files and spawn async tasks using tokio::spawn
    for file in files.iter() {
        let token = token.to_string();
        let file = file.clone(); // Clone the file to avoid lifetime issues
        tasks.push(tokio::spawn(async move {
            let push_events = parse_push_events(&file);
            filter_solidity_repos(push_events, &token).await
        }));
    }

    // Wait for all tasks to complete
    let results = join_all(tasks).await;

    let mut hardhat_repos = HashSet::new();
    let mut foundry_repos = HashSet::new();

    // Collect the results from all tasks
    for result in results {
        if let Ok((hh_repos, fd_repos)) = result {
            hardhat_repos.extend(hh_repos);
            foundry_repos.extend(fd_repos);
        }
    }

    Ok((hardhat_repos, foundry_repos))
}

