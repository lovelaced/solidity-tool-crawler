use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

pub async fn check_repo_for_file(repo_name: &str, file_name: &str, token: &str) -> bool {
    let url = format!("https://api.github.com/repos/{}/contents/{}", repo_name, file_name);
    let client = Client::new();

    let mut attempts = 0;
    loop {
        let response = client
            .get(&url)
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "solidity-tool-checker") // GitHub requires a user-agent
            .send()
            .await;

        match response {
            Ok(resp) => {
                // Check for success
                if resp.status().is_success() {
                    return true; // File found, return true
                }

                // Check for 404 (file not found), no need to retry
                if resp.status() == 404 {
                    return false; // File doesn't exist, stop retrying
                }

                // Check if rate-limited (status code 403 with rate-limiting headers)
                if resp.status() == 403 {
                    if let Some(rate_limit_remaining) = resp.headers().get("x-ratelimit-remaining") {
                        let remaining = rate_limit_remaining.to_str().unwrap_or("0").parse::<u32>().unwrap_or(0);

                        if remaining == 0 {
                            if let Some(rate_limit_reset) = resp.headers().get("x-ratelimit-reset") {
                                // GitHub API time is in UNIX timestamp format, so we calculate how long to sleep
                                let reset_time = rate_limit_reset.to_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
                                let current_time = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();

                                let sleep_duration = if reset_time > current_time {
                                    reset_time - current_time
                                } else {
                                    60 // fallback to 60 seconds if there's a timing error
                                };

                                println!("Rate limit exceeded. Sleeping for {} seconds...", sleep_duration);
                                sleep(Duration::from_secs(sleep_duration)).await;
                            }
                        }
                    }
                }

                // If other failure, retry up to 3 times
                attempts += 1;
                if attempts >= 3 {
                    eprintln!("Failed to fetch {} after 3 attempts", url);
                    return false;
                }

                // Backoff in case of network issues or transient API errors
                println!("Retrying {} (attempt {}/3)...", url, attempts + 1);
                sleep(Duration::from_secs(attempts * 2)).await;
            }
            Err(e) => {
                eprintln!("Error making request to {}: {}. Retrying...", url, e);
                attempts += 1;

                if attempts >= 3 {
                    eprintln!("Failed to fetch {} after 3 attempts due to errors", url);
                    return false;
                }

                // Exponential backoff for network errors
                println!("Retrying {} (attempt {}/3)...", url, attempts + 1);
                sleep(Duration::from_secs(attempts * 2)).await;
            }
        }
    }
}

