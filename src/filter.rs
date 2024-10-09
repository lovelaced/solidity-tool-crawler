use serde_json::Value;
use std::collections::HashSet;
use crate::github_api::check_repo_for_file;

pub async fn filter_solidity_repos(
    push_events: Vec<Value>,
    token: &str,
) -> (HashSet<String>, HashSet<String>) {
    let mut hardhat_repos = HashSet::new();
    let mut foundry_repos = HashSet::new();

    // A set to track repos that require API checking
    let mut repos_to_check = HashSet::new();

    // Solidity-related terms to look for in commit messages
    let solidity_keywords = [
        "solidity", "contract", "sol", "evm", "hardhat", "foundry",
        "pragma", "ethereum", "yul", "sc",
        "smart contract", "erc20", "erc721", "erc1155",
        "openzeppelin", "metamask", "gas", "abi", "bytecode",
        "ethers.js", "web3.js", "truffle", "solc",
        "delegatecall", "multisig", "wallet"
    ];

    // Convert the Solidity keywords to a HashSet for faster lookups
    let keyword_set: HashSet<&str> = solidity_keywords.iter().cloned().collect();

    println!("Starting to process push events...");

    // Process each push event
    for event in push_events.iter() {
        // Extract owner and repo names
        let repo_full_name = event["repo"]["name"].as_str().unwrap_or("Unknown/Unknown").to_string();
        let parts: Vec<&str> = repo_full_name.split('/').collect();

        // Ensure we have both owner and repo names
        if parts.len() != 2 {
            println!("Invalid repo name format: {}", repo_full_name);
            continue;
        }

        let owner_name = parts[0].to_string();
        let repo_name = parts[1].to_string();

        // Track whether this repo has Solidity-related commit messages
        let mut has_solidity_related_commit = false;
        let mut is_hardhat_repo = false;
        let mut is_foundry_repo = false;

        // Check the commit messages for Solidity-related terms
        for commit in event["payload"]["commits"].as_array().unwrap_or(&vec![]) {
            if let Some(message) = commit["message"].as_str() {
                // Store the lowercase version of the message in a variable
                let lowercased_message = message.to_lowercase();

                // Split the commit message into lowercase words
                let words: HashSet<&str> = lowercased_message
                    .split_whitespace()
                    .collect();

                // Check if any Solidity-related keyword is in the commit message
                if !words.is_disjoint(&keyword_set) {
                    println!("Processing commit with Solidity-related message: \"{}\" in repo {}/{}", message, owner_name, repo_name);
                    has_solidity_related_commit = true;
                }

                // Check for Hardhat or Foundry files in the "added" files list
                if let Some(added) = commit["added"].as_array() {
                    for file in added {
                        let file_name = file.as_str().unwrap();
                        if file_name.ends_with("hardhat.config.js") || file_name.ends_with("hardhat.config.ts") {
                            is_hardhat_repo = true;
                            hardhat_repos.insert(repo_full_name.clone());
                            println!("Found Hardhat config in repo {}/{}", owner_name, repo_name);
                        }

                        if file_name.ends_with("foundry.toml") {
                            is_foundry_repo = true;
                            foundry_repos.insert(repo_full_name.clone());
                            println!("Found Foundry config in repo {}/{}", owner_name, repo_name);
                        }
                    }
                }
            }
        }

        // Only consider marking the repo for API checks if there's a Solidity-related commit
        if has_solidity_related_commit {
            // If neither Hardhat nor Foundry config was found locally, flag the repo for API checks
            if !is_hardhat_repo && !is_foundry_repo {
                repos_to_check.insert(repo_full_name.clone());
                println!("Marking repo {}/{} for API checks", owner_name, repo_name);
            }
        }
    }

    // Only query the API for repos that were flagged for missing Hardhat or Foundry files
    println!("Starting API checks for flagged repos...");
    for repo_name in repos_to_check {
        let mut found_anything = false;

        if !hardhat_repos.contains(&repo_name) {
            if check_repo_for_file(&repo_name, "hardhat.config.js", token).await
                || check_repo_for_file(&repo_name, "hardhat.config.ts", token).await
            {
                hardhat_repos.insert(repo_name.clone());
                println!("Hardhat config found via API in repo {}", repo_name);
                found_anything = true;
            }
        }

        if !foundry_repos.contains(&repo_name) {
            if check_repo_for_file(&repo_name, "foundry.toml", token).await {
                foundry_repos.insert(repo_name.clone());
                println!("Foundry config found via API in repo {}", repo_name);
                found_anything = true;
            }
        }

        // Print "Nothing found" message if no relevant files were found
        if !found_anything {
            println!("Nothing found in {}", repo_name);
        }
    }

    // Final summary output
    println!("Finished processing push events.");
    println!("Total Hardhat Repos detected: {}", hardhat_repos.len());
    println!("Total Foundry Repos detected: {}", foundry_repos.len());

    (hardhat_repos, foundry_repos)
}

