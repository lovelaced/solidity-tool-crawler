use serde_json::Value;
use std::collections::HashSet;

pub async fn filter_solidity_repos(
    push_events: Vec<Value>,
) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
    let mut hardhat_repos = HashSet::new();
    let mut foundry_repos = HashSet::new();

    // A set to track repos that require API checking
    let mut repos_to_check = HashSet::new();

    // Solidity-related terms to look for in commit messages
    let solidity_keywords = [
        "solidity", "contract", "sol", "evm", "hardhat", "foundry",
        "pragma", "yul", "sc",
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

                        // Allow for both Hardhat and Foundry to be marked for the same repo
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
            // Mark the repo for API checks if neither Hardhat nor Foundry config was found locally
            if !is_hardhat_repo || !is_foundry_repo {
                repos_to_check.insert(repo_full_name.clone());
                println!("Marking repo {}/{} for API checks", owner_name, repo_name);
            }

            // Indicate if a repo uses both Hardhat and Foundry
            if is_hardhat_repo && is_foundry_repo {
                println!("Repo {}/{} uses both Hardhat and Foundry", owner_name, repo_name);
            }
        }
    }

    // Return repos to check, local Hardhat repos, and local Foundry repos
    (repos_to_check, hardhat_repos, foundry_repos)
}

