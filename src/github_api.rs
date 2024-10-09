use reqwest::Client;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

pub async fn check_repo_for_files(repo_name: &str, token: &str) -> Result<(bool, bool), Box<dyn Error + Send + Sync>> {
    let url = format!("https://api.github.com/repos/{}/contents/", repo_name);
    let client = Client::new();
    
    let mut attempts = 0;

    while attempts < 3 {
        let response = client
            .get(&url)
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "solidity-tool-checker") // GitHub requires a user-agent
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    let files = resp.json::<Vec<serde_json::Value>>().await?;

                    let mut found_hardhat = false;
                    let mut found_foundry = false;

                    for file in files {
                        if let Some(file_name) = file["name"].as_str() {
                            if file_name == "hardhat.config.js" || file_name == "hardhat.config.ts" {
                                found_hardhat = true;
                            } else if file_name == "foundry.toml" {
                                found_foundry = true;
                            }
                        }
                    }

                    return Ok((found_hardhat, found_foundry));
                }

                if resp.status() == 404 {
                    println!("Repo not found (404): {}", repo_name);
                    return Ok((false, false));
                }

                if resp.status() == 403 {
                    if let Some(rate_limit_remaining) = resp.headers().get("x-ratelimit-remaining") {
                        let remaining = rate_limit_remaining.to_str().unwrap_or("0").parse::<u32>().unwrap_or(0);

                        if remaining == 0 {
                            if let Some(rate_limit_reset) = resp.headers().get("x-ratelimit-reset") {
                                let reset_time = rate_limit_reset.to_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
                                let current_time = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();

                                let sleep_duration = if reset_time > current_time {
                                    reset_time - current_time
                                } else {
                                    60
                                };

                                println!("Rate limit exceeded. Sleeping for {} seconds...", sleep_duration);
                                sleep(Duration::from_secs(sleep_duration)).await;
                            }
                        }
                    }
                }

                attempts += 1;
                if attempts >= 3 {
                    eprintln!("Failed to fetch repo {} after 3 attempts", repo_name);
                    return Ok((false, false));
                }

                sleep(Duration::from_secs(attempts * 2)).await;
            }
            Err(e) => {
                eprintln!("Error making request to {}: {}. Retrying...", repo_name, e);
                attempts += 1;

                if attempts >= 3 {
                    eprintln!("Failed to fetch repo {} after 3 attempts due to errors", repo_name);
                    return Ok((false, false));
                }

                sleep(Duration::from_secs(attempts * 2)).await;
            }
        }
    }

    Ok((false, false))
}

