use std::{collections::VecDeque, process, sync::Arc, time::Duration};

use anyhow::Result;
use clap::Parser;

use crawler::{crawl, CrawlerState, CrawlerStateRef};
use log::info;
use tokio::{sync::RwLock, task::JoinSet};

mod crawler;

/// Program arguments
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct ProgramArgs {
    /// Initial search URL
    #[arg(short, long)]
    starting_url: String,

    /// Maximum links to find
    #[arg(short, long, default_value_t = 100)]
    max_links: u64,

    /// Number of worker threads
    #[arg(short, long, default_value_t = 4)]
    n_workers_threads: u64,

    /// Enable logging the current status
    #[arg(short, long, default_value_t = false)]
    log_status: bool,
}

async fn output_status(crawler_state: CrawlerStateRef) -> Result<()> {
    loop {
        let link_queue = crawler_state.link_queue.read().await;
        let already_visited = crawler_state.already_visited.read().await;

        if already_visited.len() > crawler_state.max_links {
            // Show the links
            // println!("All links found: {:#?}", already_visited);
            break Ok(());
        }

        println!("Number of links visited: {}", already_visited.len());
        println!("Number of links in the queue: {}", link_queue.len());

        drop(link_queue);
        drop(already_visited);

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn try_main(args: ProgramArgs) -> Result<()> {
    // call crawl(...)
    let crawler_state = CrawlerState {
        link_queue: RwLock::new(VecDeque::from([args.starting_url])),
        already_visited: RwLock::new(Default::default()),
        max_links: args.max_links as usize,
    };
    let crawler_state = Arc::new(crawler_state);

    // The actual crawling goes here
    let mut tasks = JoinSet::new();

    // Add as many crawling workers as the user has specified
    for _ in 0..args.n_workers_threads {
        let crawler_state = crawler_state.clone();
        let task = tokio::spawn(async move { crawl(crawler_state).await });

        tasks.spawn(task);
    }

    if args.log_status {
        let crawler_state = crawler_state.clone();
        tasks.spawn(tokio::spawn(
            async move { output_status(crawler_state).await },
        ));
    }

    while let Some(result) = tasks.join_next().await {
        if let Err(e) = result {
            log::error!("Error: {:?}", e);
        }
    }
    // Finished Crawling

    let already_visited = crawler_state.already_visited.read().await;
    println!("{:#?}", already_visited);

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("starting up");

    let args = ProgramArgs::parse();

    // println!("{:?}", args);

    match try_main(args).await {
        Ok(_) => log::info!("Finished"),
        Err(e) => {
            log::error!("Error: {:?}", e);
            process::exit(-1);
        }
    }
}

/* DEMO TEST:

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct Response {
    #[serde(rename = "userId")]
    pub user_id: i32,
    pub id: i32,
    pub title: String,
    pub body: String,
}

let resp = reqwest::get("https://jsonplaceholder.typicode.com/posts")
    .await?
    .json::<Vec<Response>>()
    .await?;
println!("{resp:#?}");

*/
