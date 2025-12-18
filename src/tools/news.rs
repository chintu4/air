use super::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn};
use scraper::{Html, Selector};

pub struct NewsTool {
    client: Client,
}

impl NewsTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .unwrap();

        Self { client }
    }

    async fn scrape_google_news(&self, max_articles: usize) -> Result<Vec<Value>> {
        let url = "https://news.google.com";
        info!("Fetching Google News from {}", url);

        let response = self.client.get(url).send().await?;
        let html_content = response.text().await?;
        let document = Html::parse_document(&html_content);

        // Use the specific selector from the Python script: 'a.DY5T1d'
        let selector = Selector::parse("a.DY5T1d").map_err(|e| anyhow!("Failed to parse selector: {:?}", e))?;

        let mut articles = Vec::new();

        for element in document.select(&selector).take(max_articles) {
            let title = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
            let href = element.value().attr("href").unwrap_or("");

            let link = if href.starts_with("./") {
                format!("https://news.google.com{}", &href[1..])
            } else if href.starts_with("/") {
                format!("https://news.google.com{}", href)
            } else {
                href.to_string()
            };

            if !title.is_empty() {
                articles.push(json!({
                    "title": title,
                    "link": link,
                    "source": "Google News"
                }));
            }
        }

        Ok(articles)
    }
}

#[async_trait]
impl Tool for NewsTool {
    fn name(&self) -> &str {
        "WebScraper"
    }

    fn description(&self) -> &str {
        "Scrape news headlines from Google News. Useful for getting current events and top stories."
    }

    fn available_functions(&self) -> Vec<String> {
        vec!["scrape_news".to_string()]
    }

    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "scrape_news" => {
                let max_articles = args["max_articles"].as_u64().unwrap_or(10) as usize;

                match self.scrape_google_news(max_articles).await {
                    Ok(articles) => {
                         Ok(ToolResult {
                            success: true,
                            result: json!(articles),
                            metadata: Some(json!({
                                "count": articles.len(),
                                "source": "Google News"
                            })),
                        })
                    },
                    Err(e) => {
                        warn!("Failed to scrape news: {}", e);
                         Ok(ToolResult {
                            success: false,
                            result: json!(format!("Failed to scrape news: {}", e)),
                            metadata: None,
                        })
                    }
                }
            }
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}
