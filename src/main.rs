use anyhow::{Error, Ok, Result};
use osmpbfreader::{OsmPbfReader, Tags};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use std::fmt::Display;
use std::fs::File;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::macros::support::Future;
use tokio::sync::mpsc::{channel, Sender};
use tokio::task::{JoinHandle, JoinSet};

extern crate osmpbfreader;

#[tokio::main(flavor = "multi_thread", worker_threads = 40)]
async fn main() -> Result<()> {
    parse_osm().await
}

async fn parse_osm() -> Result<()> {
    let osm_file = File::open("src/osm-eu/europe-latest.osm.pbf")?;
    let mut pbf = OsmPbfReader::new(osm_file);
    let (sender, channel_handle) = start_channel().await?;
    let client = build_client()?;
    let mut join_set = JoinSet::new();

    for obj in pbf.par_iter() {
        if obj.is_err() {
            continue;
        }

        let obj = obj.unwrap();
        let tags = obj.tags();

        let mut contains_museum = false;
        let mut contains_website = false;

        for tag in tags.iter() {
            if tag.1.to_string().contains_museum() {
                contains_museum = true;
            } else if tag.1.to_string().contains_link() {
                contains_website = true;
            }
        }

        if !contains_museum || !contains_website {
            continue;
        }

        join_set.spawn(process_museum(client.clone(), sender.clone(), tags.into()).await);
    }

    let errors = join_set
        .join_next()
        .await
        .into_iter()
        .filter_map(|t| {
            if t.is_err() || t.unwrap().is_err() {
                Some(())
            } else {
                None
            }
        })
        .count();

    sender.send(Message::Close).await?;
    channel_handle.await?;

    println!("{} error/s occured while processing museums.", errors);

    Ok(())
}

async fn start_channel() -> Result<(Sender<Message>, JoinHandle<()>), Error> {
    let (sender, mut receiver) = channel::<Message>(10000);

    let handle = tokio::spawn(async move {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("museum_data_eu.txt")
            .await
            .expect("Error creating file.");

        let mut writer = BufWriter::new(file);

        while let Some(message) = receiver.recv().await {
            match message {
                Message::Data(s) => {
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

fn build_client() -> Result<Client, Error> {
    let mut headers = HeaderMap::new();

    headers.append(
        "User-Agent",
        HeaderValue::from_str(
            "Mozilla/5.0 (Android 14; Mobile; rv:129.0) Gecko/129.0 Firefox/129.0",
        )
        .unwrap(),
    );

    return reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .default_headers(headers)
        .build()
        .map_err(Error::from);
}

async fn process_museum(
    client: Client,
    sender: Sender<Message>,
    valid_tags: ValidTags,
) -> impl Future<Output = Result<(), Error>> {
    return async move {
        let museum: Museum = valid_tags.into();

        if museum.websites_contain_art(client).await {
            sender.send(Message::Data(museum.to_string())).await?;
        }

        Ok(())
    };
}

enum Message {
    Data(String),
    Close,
}

struct Museum {
    names: Vec<String>,
    adr: Vec<String>,
    other: Vec<String>,
}

impl Museum {
    fn add_name(&mut self, new: String) {
        self.names.push(new)
    }

    fn add_adr(&mut self, new: String) {
        self.adr.push(new)
    }

    fn add_other(&mut self, new: String) {
        self.other.push(new)
    }

    async fn websites_contain_art(&self, client: Client) -> bool {
        let mut join_set = JoinSet::new();

        self.other.iter().for_each(|url| {
            join_set.spawn(client.get(url).send());
        });

        for res in join_set.join_next().await.into_iter() {
            if res.is_err() || res.as_ref().unwrap().is_err() {
                continue;
            }

            if res
                .unwrap()
                .unwrap()
                .text()
                .await
                .is_ok_and(|s| s.contains_art())
            {
                join_set.abort_all();
                return true;
            }
        }

        return false;
    }
}

impl Default for Museum {
    fn default() -> Self {
        Self {
            names: Vec::new(),
            adr: Vec::new(),
            other: Vec::new(),
        }
    }
}

impl From<ValidTags> for Museum {
    fn from(value: ValidTags) -> Self {
        let mut museum = Self::default();

        value.data.into_iter().for_each(|tag| {
            if tag.key.contains_name() {
                museum.add_name(tag.value);
            } else if tag.key.contains_adr() {
                museum.add_adr(tag.value);
            } else {
                museum.add_other(tag.value);
            }
        });

        return museum;
    }
}

impl Display for Museum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: {:?}\nAdresse: {:?}\nAnderes: {:?}\n\n",
            self.names, self.adr, self.other,
        )
    }
}

struct Tag {
    key: String,
    value: String,
}

struct ValidTags {
    data: Vec<Tag>,
}

impl From<&Tags> for ValidTags {
    fn from(value: &Tags) -> Self {
        let data = value
            .iter()
            .filter_map(|tag| {
                let key = tag.0.to_lowercase();
                let value = tag.1.to_lowercase();

                return if key.contains_name()
                    || key.contains_adr()
                    || key.contains_link()
                    || key.contains_other()
                    || value.contains_link()
                {
                    Some(Tag { key, value })
                } else {
                    None
                };
            })
            .collect();

        return Self { data };
    }
}

trait ContainsOSM {
    fn contains_art(&self) -> bool;
    fn contains_link(&self) -> bool;
    fn contains_name(&self) -> bool;
    fn contains_other(&self) -> bool;
    fn contains_adr(&self) -> bool;
    fn contains_museum(&self) -> bool;
}

impl ContainsOSM for String {
    fn contains_art(&self) -> bool {
        let lowercase = self.to_lowercase();

        return lowercase.split(" ").map(|v| v.trim()).any(|s| {
            s == "art"
                || s == "изкуство"
                || s == "kunst"
                || s == "taide"
                || s == "τέχνη"
                || s == "Ealaín"
                || s == "gr"
                || s == "arte"
                || s == "קונסט"
                || s == "umjetnost"
                || s == "чл"
                || s == "sztuka"
                || s == "artă"
                || s == "искусство"
                || s == "konst"
                || s == "уметност"
                || s == "čl"
                || s == "Umetnost"
                || s == "umění"
                || s == "ст"
                || s == "művészet"
                || s == "celf"
                || s == "арт"
        });
    }

    fn contains_link(&self) -> bool {
        let lc = self.to_lowercase();

        return (lc.contains("website")
            || lc.contains("http")
            || lc.contains("https")
            || lc.contains("www.")
            || lc.contains(".com")
            || lc.contains(".de")
            || lc.contains(".at")
            || lc.contains(".uk")
            || lc.contains(".eu")
            || lc.contains(".it")
            || lc.contains(".by")
            || lc.contains(".ch")
            || lc.contains(".cz")
            || lc.contains(".am")
            || lc.contains(".bg")
            || lc.contains(".dk")
            || lc.contains(".ee")
            || lc.contains(".es")
            || lc.contains(".fi")
            || lc.contains(".fr")
            || lc.contains(".gl")
            || lc.contains(".gr")
            || lc.contains(".hr")
            || lc.contains(".hu")
            || lc.contains(".ie")
            || lc.contains(".is")
            || lc.contains(".je")
            || lc.contains(".li")
            || lc.contains(".lt")
            || lc.contains(".lu")
            || lc.contains(".lv")
            || lc.contains(".mc")
            || lc.contains(".md")
            || lc.contains(".me")
            || lc.contains(".nl")
            || lc.contains(".no")
            || lc.contains(".pl")
            || lc.contains(".pt")
            || lc.contains(".ro")
            || lc.contains(".rs")
            || lc.contains(".se")
            || lc.contains(".si")
            || lc.contains(".sk")
            || lc.contains(".ua"))
            && !lc.contains("@");
    }

    fn contains_name(&self) -> bool {
        return self.to_lowercase().contains("name");
    }

    fn contains_other(&self) -> bool {
        return self.to_lowercase().contains("contact");
    }

    fn contains_adr(&self) -> bool {
        let lc = self.to_lowercase();
        return lc.contains("city") || lc.contains("country");
    }

    fn contains_museum(&self) -> bool {
        return self.to_lowercase().contains("museum");
    }
}
