mod contains_osm;
mod message;
mod museum;
mod tag;

use anyhow::{Error, Ok, Result};
use contains_osm::ContainsOSM;
use message::Message;
use museum::Museum;
use osmpbfreader::OsmPbfReader;
use reqwest::Client;
use std::fs::File;
use std::time::Duration;
use tag::Tag;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::macros::support::Future;
use tokio::sync::mpsc::{channel, Sender};
use tokio::task::{JoinError, JoinHandle};

extern crate osmpbfreader;

#[tokio::main(flavor = "multi_thread", worker_threads = 20)]
async fn main() -> Result<()> {
    parse_osm().await
}

async fn parse_osm() -> Result<()> {
    let osm_file = File::open("src/osm-eu/europe-latest.osm.pbf")?;
    let mut pbf = OsmPbfReader::new(osm_file);
    let mut objs = pbf.iter().map(Result::unwrap);

    let (sender, channel_handle) = start_channel().await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let mut handles: Vec<JoinHandle<Result<(), Error>>> = Vec::new();

    while let Some(obj) = objs.next() {
        if obj
            .tags()
            .iter()
            .any(|tag| tag.1.to_lowercase().contains_museum())
            && obj
                .tags()
                .iter()
                .any(|tag| tag.1.to_string().contains_link())
        {
            let valid_tags: Vec<Tag> = obj
                .tags()
                .iter()
                .filter_map(|tag| {
                    let key = tag.0.to_lowercase();
                    let value = tag.1.to_lowercase();
                    if key.contains_name()
                        || key.contains_adr()
                        || key.contains_link()
                        || key.contains_other()
                        || value.contains_link()
                    {
                        Some(Tag { key, value })
                    } else {
                        None
                    }
                })
                .collect();

            handles.push(tokio::spawn(
                process_museum(client.clone(), sender.clone(), valid_tags).await,
            ));
        }
    }

    let mut results: Vec<Result<Result<(), Error>, JoinError>> = Vec::new();

    for handle in handles {
        results.push(handle.await);
    }

    let errors = results
        .into_iter()
        .filter(|r| r.is_err() || r.as_ref().unwrap().is_err())
        .count();

    if errors == 0 {
        println!("No errors occured while processing museums.")
    } else {
        println!("{} error/s occured while processing museums.", errors)
    }

    sender.send(Message::Close).await?;
    channel_handle.await?;

    Ok(())
}

async fn start_channel() -> Result<(Sender<Message>, JoinHandle<()>), Error> {
    let (sender, mut receiver) = channel::<Message>(10000);

    let handle = tokio::spawn(async move {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .create(true)
            .open("museum_data_eu.txt")
            .await
            .expect("Error creating file.");

        let mut writer = BufWriter::new(file);

        while let Some(message) = receiver.recv().await {
            match message {
                Message::Text(s) => {
                    writer
                        .write_all(&s.into_bytes())
                        .await
                        .expect("Error wrinting.");
                }
                Message::Close => receiver.close(),
            }
        }

        writer.flush().await.expect("Error flushing.");
    });

    Ok((sender, handle))
}

async fn process_museum(
    client: Client,
    sender: Sender<Message>,
    valid_tags: Vec<Tag>,
) -> impl Future<Output = Result<(), Error>> {
    return async move {
        let mut museum = Museum::default();

        for pair in valid_tags.into_iter() {
            if pair.key.contains_name() {
                museum.add_name(pair.value);
            } else if pair.key.contains_adr() {
                museum.add_adr(pair.value);
            } else {
                museum.add_other(pair.value);
            }
        }

        if websites_contain_art(client, &museum.other).await {
            sender.send(Message::Text(museum.to_string())).await?;
        }

        Ok(())
    };
}

async fn websites_contain_art(client: Client, urls: &Vec<String>) -> bool {
    for url in urls {
        let request_result = client.get(url).send().await;

        if request_result.is_err() {
            continue;
        }

        let text_result = request_result.unwrap().text().await;

        if text_result.is_ok_and(|s| s.contains_art()) {
            return true;
        }
    }

    false
}
