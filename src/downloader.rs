use reqwest::Client;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use chrono::{DateTime, Utc, Duration};
use std::fs::{create_dir_all, metadata};
use std::error::Error;
use futures::future::join_all;
use tokio::time::sleep;
use std::time::Duration as StdDuration;

async fn download_file(url: &str, path: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let response = client.get(url).send().await?;
    
    let mut file = File::create(path).await?;
    let content = response.bytes().await?;
    file.write_all(&content).await?;
    
    Ok(())
}

pub async fn download_gharchive_data(start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut current_date = start_date;
    create_dir_all("gharchive_data")?;

    while current_date <= end_date {
        let mut tasks = vec![];
        for hour in 0..24 {
            let url = format!("https://data.gharchive.org/{}-{}.json.gz", current_date.format("%Y-%m-%d"), hour);
            let path = format!("gharchive_data/{}-{}.json.gz", current_date.format("%Y-%m-%d"), hour);

            // Check if the file already exists
            if metadata(&path).is_ok() {
                println!("File already exists, skipping download: {}", path);
                continue;  // Skip the download if the file already exists
            }

            println!("Downloading: {}", url);

            tasks.push(tokio::spawn(async move {
                for attempt in 0..3 {
                    if let Err(e) = download_file(&url, &path).await {
                        eprintln!("Failed to download {}: {} (attempt {})", url, e, attempt + 1);
                        sleep(StdDuration::from_secs(2 * (attempt + 1) as u64)).await; // Backoff strategy
                    } else {
                        break;
                    }
                }
            }));
        }
        join_all(tasks).await;
        current_date = current_date + Duration::days(1);
    }
    
    Ok(())
}

