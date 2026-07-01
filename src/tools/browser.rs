use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use crate::tools::{Tool, ToolContext};

pub struct BrowserReadTool;

#[async_trait::async_trait]
impl Tool for BrowserReadTool {
    fn name(&self) -> &str {
        "browser_read"
    }

    fn description(&self) -> &str {
        "Fetch the text contents of a web page URL and convert it into clean Markdown. Always safe to run."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL of the website to retrieve content from (e.g. 'https://docs.rs/tokio/')."
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<String> {
        let url = args["url"].as_str().ok_or_else(|| anyhow!("Missing 'url' argument"))?;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            return Ok(format!("Error fetching URL: HTTP {}", resp.status()));
        }

        let html = resp.text().await?;

        // Simple HTML-to-text extraction using the scraper crate
        let document = scraper::Html::parse_document(&html);
        
        // Extract page title
        let title_selector = scraper::Selector::parse("title").unwrap();
        let title = document.select(&title_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| "Untitled Page".to_string());

        // Extract paragraphs and headings
        let content_selector = scraper::Selector::parse("h1, h2, h3, p").unwrap();
        let mut markdown = format!("# {}\n\n", title);

        for element in document.select(&content_selector) {
            let tag_name = element.value().name();
            let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if text.is_empty() {
                continue;
            }
            match tag_name {
                "h1" => markdown.push_str(&format!("\n# {}\n", text)),
                "h2" => markdown.push_str(&format!("\n## {}\n", text)),
                "h3" => markdown.push_str(&format!("\n### {}\n", text)),
                "p" => markdown.push_str(&format!("{}\n\n", text)),
                _ => {}
            }
        }

        if markdown.len() > 20_000 {
            markdown.truncate(20_000);
            markdown.push_str("\n\n... [truncated, page content exceeds 20,000 characters]");
        }

        Ok(markdown)
    }
}
