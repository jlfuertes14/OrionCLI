use crate::tools::{Tool, ToolContext};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

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
        let url = args["url"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'url' argument"))?;

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
        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| "Untitled Page".to_string());

        // Extract paragraphs and headings
        let content_selector = scraper::Selector::parse("h1, h2, h3, p").unwrap();
        let mut markdown = format!("# {}\n\n", title);

        for element in document.select(&content_selector) {
            let tag_name = element.value().name();
            let text = element
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
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

pub struct WebSearchTool;

#[async_trait::async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Perform a web search for a given query and return top results with titles, snippets, and URLs."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query (e.g. 'rust async lifetimes')."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<String> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'query' argument"))?;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        let mut provider = _ctx.settings.search_provider.clone();
        let mut tavily_key = std::env::var("TAVILY_API_KEY")
            .ok()
            .or_else(|| _ctx.settings.tavily_api_key.clone());

        if std::env::var("TAVILY_API_KEY").is_ok() {
            provider = Some(crate::config::SearchProvider::Tavily);
        }

        if provider.is_none() && tavily_key.is_none() {
            use std::io::{IsTerminal, Write};
            if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
                use colored::Colorize;
                eprintln!("\n{}", "Orion Web Search Setup:".bold().cyan());
                eprintln!("Choose a web search provider:");
                eprintln!("  1. Tavily Search API (Requires API key, rich structured content)");
                eprintln!("  2. DuckDuckGo Lite (Free, zero setup, fallback scraping)");

                let mut choice = String::new();
                loop {
                    eprint!("Select [1 or 2]: ");
                    let _ = std::io::stderr().flush();
                    choice.clear();
                    if std::io::stdin().read_line(&mut choice).is_err() {
                        break;
                    }
                    let clean = choice.trim();
                    if clean == "1" {
                        provider = Some(crate::config::SearchProvider::Tavily);

                        let mut key = String::new();
                        eprint!("Enter your Tavily API Key: ");
                        let _ = std::io::stderr().flush();
                        if std::io::stdin().read_line(&mut key).is_ok() {
                            let clean_key = key.trim().to_string();
                            if !clean_key.is_empty() {
                                tavily_key = Some(clean_key.clone());
                                let mut new_settings = _ctx.settings.clone();
                                new_settings.search_provider =
                                    Some(crate::config::SearchProvider::Tavily);
                                new_settings.tavily_api_key = Some(clean_key);
                                let _ = new_settings.save();
                            }
                        }
                        break;
                    } else if clean == "2" {
                        provider = Some(crate::config::SearchProvider::DuckDuckGo);
                        let mut new_settings = _ctx.settings.clone();
                        new_settings.search_provider =
                            Some(crate::config::SearchProvider::DuckDuckGo);
                        let _ = new_settings.save();
                        break;
                    } else {
                        eprintln!("Invalid choice. Please enter 1 or 2.");
                    }
                }
            } else {
                provider = Some(crate::config::SearchProvider::DuckDuckGo);
            }
        }

        if provider == Some(crate::config::SearchProvider::Tavily) || tavily_key.is_some() {
            if let Some(key) = tavily_key {
                let req_body = serde_json::json!({
                    "api_key": key,
                    "query": query,
                    "search_depth": "basic",
                    "max_results": 5
                });
                let resp = client
                    .post("https://api.tavily.com/search")
                    .json(&req_body)
                    .send()
                    .await?;
                if !resp.status().is_success() {
                    return Ok(format!(
                        "Error performing Tavily search: HTTP {}",
                        resp.status()
                    ));
                }
                let data: Value = resp.json().await?;
                if let Some(results) = data["results"].as_array() {
                    let mut out = Vec::new();
                    for r in results {
                        let title = r["title"].as_str().unwrap_or("Untitled");
                        let url = r["url"].as_str().unwrap_or("");
                        let content = r["content"].as_str().unwrap_or("");
                        out.push(format!("### [{}]({})\n{}\n", title, url, content));
                    }
                    if out.is_empty() {
                        return Ok("No results found.".to_string());
                    }
                    return Ok(out.join("\n"));
                } else {
                    return Ok("No results found or invalid Tavily response.".to_string());
                }
            }
        }

        // Fallback to DuckDuckGo Lite
        let params = [("q", query)];
        let search_url = "https://lite.duckduckgo.com/lite/";
        let resp = client.post(search_url).form(&params).send().await?;

        if !resp.status().is_success() {
            return Ok(format!("Error performing search: HTTP {}", resp.status()));
        }

        let html = resp.text().await?;
        let document = scraper::Html::parse_document(&html);

        let sel_title = scraper::Selector::parse(".result-link").unwrap();
        let sel_snippet = scraper::Selector::parse(".result-snippet").unwrap();

        let titles: Vec<_> = document.select(&sel_title).collect();
        let snippets: Vec<_> = document.select(&sel_snippet).collect();

        let mut results = Vec::new();
        for (i, title_el) in titles.iter().take(5).enumerate() {
            let title = title_el
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            let href = title_el.value().attr("href").unwrap_or("").to_string();
            let snippet = if i < snippets.len() {
                snippets[i]
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string()
            } else {
                String::new()
            };

            results.push(format!("### [{}]({})\n{}\n", title, href, snippet));
        }

        if results.is_empty() {
            Ok("No results found.".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }
}

fn percent_encode(query: &str) -> String {
    let mut encoded = String::new();
    for b in query.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => {
                encoded.push('+');
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

pub struct OpenBrowserTool;

#[async_trait::async_trait]
impl Tool for OpenBrowserTool {
    fn name(&self) -> &str {
        "open_browser"
    }

    fn description(&self) -> &str {
        "Open a given URL in the user's default system web browser. Use this when the user asks you to open a website on their machine."
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
                    "description": "The URL to open (e.g. 'https://www.youtube.com')."
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<String> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'url' argument"))?;

        if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(&["/C", "start", "", url])
                .spawn()?;
        } else if cfg!(target_os = "macos") {
            std::process::Command::new("open").arg(url).spawn()?;
        } else {
            std::process::Command::new("xdg-open").arg(url).spawn()?;
        }

        Ok(format!(
            "Successfully opened {} in the default browser.",
            url
        ))
    }
}

pub struct PlayYoutubeTool;

#[async_trait::async_trait]
impl Tool for PlayYoutubeTool {
    fn name(&self) -> &str {
        "play_youtube"
    }

    fn description(&self) -> &str {
        "Search YouTube for a video/song and play it directly in the browser by finding the first video result. Use this when the user asks you to play a video or music on YouTube."
    }

    fn requires_approval(&self) -> bool {
        false
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The video search query (e.g. 'Iris Kenshi Yonezu')."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<String> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing 'query' argument"))?;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        let search_url = format!(
            "https://www.youtube.com/results?search_query={}",
            percent_encode(query)
        );
        let resp = client.get(&search_url).send().await?;

        let mut yt_url = None;
        if resp.status().is_success() {
            let html = resp.text().await?;
            if let Some(pos) = html.find("/watch?v=") {
                let watch_part = &html[pos..pos + 20];
                let mut clean_watch = String::new();
                for c in watch_part.chars() {
                    if c.is_alphanumeric()
                        || c == '/'
                        || c == '?'
                        || c == '='
                        || c == '-'
                        || c == '_'
                    {
                        clean_watch.push(c);
                    } else {
                        break;
                    }
                }
                yt_url = Some(format!("https://www.youtube.com{}", clean_watch));
            }
        }

        let final_url = match yt_url {
            Some(url) => url,
            None => {
                // Fallback to searching YouTube directly (general page) if direct link extraction failed
                search_url
            }
        };

        if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(&["/C", "start", "", &final_url])
                .spawn()?;
        } else if cfg!(target_os = "macos") {
            std::process::Command::new("open").arg(&final_url).spawn()?;
        } else {
            std::process::Command::new("xdg-open")
                .arg(&final_url)
                .spawn()?;
        }

        Ok(format!(
            "Successfully opened {} in the default browser.",
            final_url
        ))
    }
}
