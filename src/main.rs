use std::{env, process};

use anyhow::{bail, Result};
use log::info;
use reqwest::Client;
use url::Url;

mod crawler;

async fn try_main(args: Option<String>) -> Result<()> {
    if let None = args {
        bail!("arguments could not be parsed");
    };

    let url = Url::parse(args.unwrap().as_str())?;

    let client = Client::new();

    // let url = Url::parse("https://rustlang-es.org/")?;

    let links = crawler::find_links(url, &client).await;

    println!("{:#?}", links);

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("starting up");

    let args = env::args().collect::<Vec<String>>().into_iter().nth(1);

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
