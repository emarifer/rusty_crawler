use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use reqwest::{Client, StatusCode};
use scraper::Selector;
use url::Url;

const LINK_REQUEST_TIMEOUT_S: u64 = 2;

/// This will turn relative urls into
/// full urls.
/// E.g. get_url("/services/", "https://google.com/") -> "https://google.com/services/"
fn get_url(path: &str, root_url: Url) -> Result<Url> {
    if let Ok(url) = Url::parse(&path) {
        return Ok(url);
    }

    root_url
        .join(&path)
        .ok()
        .ok_or(anyhow!("could not join relative path"))
}

/// Given a `url` and a `client`, it will return the
/// parsed HTML in a DOM structure. It may return
/// an error if the request fails.
async fn get_all_links(url: Url, client: &Client) -> Result<Vec<String>> {
    let response = client
        .get(url)
        .timeout(Duration::from_secs(LINK_REQUEST_TIMEOUT_S))
        .send()
        .await?;

    if response.status() != StatusCode::OK {
        bail!("page returned invalid response");
    }

    let html = response.text().await?;

    let link_selector = Selector::parse("a").unwrap();

    Ok(scraper::Html::parse_document(&html)
        .select(&link_selector)
        .filter_map(|e| e.value().attr("href"))
        .map(|href| href.to_string())
        .collect::<Vec<String>>())
}

/// Given a `url`, and a `client`, it will crawl
/// the HTML in `url` and find all the links in the
/// page, returning them as a vector of strings
pub async fn find_links(url: Url, client: &Client) -> Vec<String> {
    // This will get all the "href" tags in all the anchors
    let links = match get_all_links(url.clone(), client).await {
        Ok(links) => links,
        Err(e) => {
            log::error!("Could not find links: {}", e);
            vec![]
        }
    };

    // Turn all links into absolute links
    links
        .iter()
        .filter_map(|l| get_url(l, url.clone()).ok())
        .map(|url| url.to_string())
        .collect()
}
