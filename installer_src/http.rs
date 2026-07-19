use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

use reqwest::{Client, StatusCode};
use serde_json::{Value, value::Index};
use smol::{
    Task,
    channel::{Receiver, Sender, unbounded},
};
use thiserror::Error;

use crate::{InstallerState, http::Error::JsonType};

const RELEASE_URL: &str =
    r"https://api.github.com/repos/ConfusionCodes/undertale-mod-manager/releases/latest";
const USER_AGENT: &str = "Undertale-Mod-Manager-Installer";

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error handling HTTP reuqest: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Request produced unexpected status code: {0}")]
    Status(StatusCode),
    #[error("Error deserializing json object: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("Json object did not contain the expected key '{1}': {0}")]
    JsonKey(Value, &'static str),
    #[error("Json array did not contain the expected index '{1}': {0}")]
    JsonIndex(Value, usize),
    #[error("Json value was not of type '{1}': {0}")]
    JsonType(Value, &'static str),
    #[error("Could not write to file: {0}")]
    IoWrite(#[from] std::io::Error),
}
trait JsonFind {
    fn key(&self, key: &'static str) -> Result<&Self, Error>;
    fn index(&self, index: usize) -> Result<&Self, Error>;
    fn get_string(&self) -> Result<String, Error>;
    fn get_u64(&self) -> Result<u64, Error>;
}
impl JsonFind for Value {
    fn key(&self, key: &'static str) -> Result<&Self, Error> {
        self.get(key)
            .ok_or_else(|| Error::JsonKey(self.clone(), key))
    }

    fn index(&self, index: usize) -> Result<&Self, Error> {
        self.get(index)
            .ok_or_else(|| Error::JsonIndex(self.clone(), index))
    }

    fn get_string(&self) -> Result<String, Error> {
        self.as_str()
            .map(|x| x.to_owned())
            .ok_or_else(|| Error::JsonType(self.clone(), "String"))
    }

    fn get_u64(&self) -> Result<u64, Error> {
        self.as_u64()
            .ok_or_else(|| Error::JsonType(self.clone(), "u64"))
    }
}

fn start_download(state: &InstallerState, path: &Path) -> (Task<Result<(), Error>>, Receiver<f32>) {
    //
    let path = path.to_path_buf();
    let (tx, rx) = unbounded();
    let task: Task<Result<(), Error>> = smol::spawn(async move {
        let client = Client::new();
        let (asset_url, asset_size) = get_asset_url(&client).await?;

        download(&client, &asset_url, &path, asset_size, tx).await
    });

    (task, rx)
}

async fn get_asset_url(client: &Client) -> Result<(String, u64), Error> {
    let response = client
        .get(RELEASE_URL)
        .header("Accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = response.status();
    if status != StatusCode::OK && status != StatusCode::FOUND {
        eprintln!("Could not find resource.");
        return Err(Error::Status(status));
    }
    let text = response.text().await?;
    let json: Value = serde_json::from_str(&text)?;
    let asset = json.key("assets")?.index(2)?;
    let url = asset.key("url")?.get_string()?;
    let size = asset.key("size")?.get_u64()?;
    Ok((url, size))
}

async fn download(
    client: &Client,
    url: &str,
    path: &Path,
    asset_size: u64,
    tx: Sender<f32>,
) -> Result<(), Error> {
    let mut response = client
        .get(url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = response.status();
    if status != StatusCode::OK && status != StatusCode::FOUND {
        eprintln!("Could not find resource.");
        return Err(Error::Status(status));
    }
    let mut downloaded: u64 = 0;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    while let Some(chunk) = response.chunk().await? {
        let chunk_size = chunk.len() as u64;
        let written_size = file.write(&chunk)? as u64;
        if written_size != chunk_size {
            eprintln!(
                "Data chunk {chunk:?} has size {chunk_size}, but only {written_size} bytes were written."
            );
        }
        downloaded += written_size;
        let percentage = downloaded as f32 / asset_size as f32;
        if let Err(err) = tx.force_send(percentage) {
            eprintln!("Faied to send percentage: {err}");
        };
    }

    Ok(())
}
// fn install() -> Result<(), reqwest::Error> {
//     let client = reqwest::blocking::Client::new();
//     let response = client
//         .get(RELEASE_URL)
//         .header("Accept", "application/json")
//         .header("User-Agent", "Undertale-Mod-Manager-Installer")
//         .send()?;
//     if response.status() != StatusCode::OK && response.status() != StatusCode::FOUND {
//         eprintln!("Could not find resource.");
//         return Ok(());
//     }
//     let data = response.text()?;
//     let result: Result<Value, serde_json::Error> = serde_json::from_str(&data);
//     let Ok(value) = result else {
//         eprintln!("Could not deserialize data.");
//         return Ok(());
//     };
//     let asset = &value["assets"][1]["url"];
//     let Value::String(asset) = asset else {
//         eprintln!("Data was not a string.");
//         return Ok(());
//     };
//     let response = client
//         .get(asset)
//         .header("Accept", "application/octet-stream")
//         .header("User-Agent", "Undertale-Mod-Manager-Installer")
//         .send()?;
//     if response.status() != StatusCode::OK && response.status() != StatusCode::FOUND {
//         eprintln!("Could not find resource 2.");
//         return Ok(());
//     }
//     let bytes = response.bytes()?;
//     let _ = std::fs::write("test.exe", bytes);

//     Ok(())
// }
