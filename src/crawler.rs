use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail, Result};
use reqwest::{Client, StatusCode};
use scraper::Selector;
use tokio::sync::RwLock;
use url::Url;

const LINK_REQUEST_TIMEOUT_S: u64 = 2;

pub struct CrawlerState {
    pub link_queue: RwLock<VecDeque<String>>,
    pub already_visited: RwLock<HashSet<String>>,
    pub max_links: usize,
}

pub type CrawlerStateRef = Arc<CrawlerState>;

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
/// page, returning them as a vector of strings.
async fn find_links(url: Url, client: &Client) -> Vec<String> {
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

/// Function to crawl links in the given url.
pub async fn crawl(crawl_state: CrawlerStateRef) -> Result<()> {
    // One client per worker thread
    let client = Client::new();

    // Crawler loop
    'crawler: loop {
        let already_visited = crawl_state.already_visited.read().await;
        if already_visited.len() > crawl_state.max_links {
            break 'crawler;
        }
        drop(already_visited);

        // Also check that max links have been reached
        let mut link_queue = crawl_state.link_queue.write().await;
        let url_str = link_queue.pop_front().unwrap_or_default();
        drop(link_queue);
        if url_str.is_empty() {
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        // Current url to visit
        let url = Url::parse(&url_str)?;

        let links = find_links(url, &client).await;

        let mut link_queue = crawl_state.link_queue.write().await;
        let mut already_visited = crawl_state.already_visited.write().await;
        for link in links {
            if !already_visited.contains(&link) {
                link_queue.push_back(link);
            }
        }

        // Add visited link to set of already visited link
        already_visited.insert(url_str);
    }

    Ok(())
}

/* QUEUE:
https://medium.com/@saverio3107/stacks-queues-lifo-vs-fifo-python-rust-04a795c495cf

Example:
https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=6c961ab444a8aa8d44e5a050f41e940b

https://blog.coolhead.in/implementing-a-queue-in-rust-using-a-vector

https://www.alxolr.com/articles/
queues-stacks-deques-data-structures-coded-in-rust

https://freedium.cfd/https://fedevitale.medium.com/thread-safe-queue-in-rust-1ed1acb9b93e

*/
