use serde_json::Value;
use smol::{
    Task,
    channel::{Receiver, Sender, bounded},
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use std::path::{Path, PathBuf};
use surf::{StatusCode, middleware::Redirect};
use thiserror::Error;

const RELEASE_URL: &str =
    r"https://api.github.com/repos/ConfusionCodes/undertale-mod-manager/releases/latest";
const USER_AGENT: &str = "Undertale-Mod-Manager-Installer";
const CHUNK_SIZE: usize = 16 * 1024;
#[derive(Debug, Error)]
pub enum Error {
    #[error("Error handling HTTP reuqest: {0}")]
    Request(surf::Error),
    #[error("Request produced unexpected status code: {0}")]
    Status(StatusCode),
    #[error("Error deserializing json object: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("Json object did not contain the expected key '{1}': {0}")]
    JsonKey(String, &'static str),
    #[error("Json array did not contain the expected index '{1}': {0}")]
    JsonIndex(String, usize),
    #[error("Json value was not of type '{1}': {0}")]
    JsonType(String, &'static str),
    #[error("Could not write to file: {0}")]
    IoWrite(#[from] std::io::Error),
}
impl From<surf::Error> for Error {
    fn from(value: surf::Error) -> Self {
        Self::Request(value)
    }
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
            .ok_or_else(|| Error::JsonKey(self.to_string(), key))
    }

    fn index(&self, index: usize) -> Result<&Self, Error> {
        self.get(index)
            .ok_or_else(|| Error::JsonIndex(self.to_string(), index))
    }

    fn get_string(&self) -> Result<String, Error> {
        self.as_str()
            .map(|x| x.to_owned())
            .ok_or_else(|| Error::JsonType(self.to_string(), "String"))
    }

    fn get_u64(&self) -> Result<u64, Error> {
        self.as_u64()
            .ok_or_else(|| Error::JsonType(self.to_string(), "u64"))
    }
}

pub fn start_download(path: PathBuf) -> (Task<Result<(), Error>>, Receiver<f32>) {
    //
    let (tx, rx) = bounded(1);
    let task: Task<Result<(), Error>> = smol::spawn(async move {
        let (asset_url, asset_size) = get_asset_url().await?;

        download(&asset_url, &path, asset_size, tx).await
    });

    (task, rx)
}

async fn get_asset_url() -> Result<(String, u64), Error> {
    let mut response = surf::get(RELEASE_URL)
        .header("Accept", "application/json")
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    let status = response.status();
    if status != StatusCode::Ok && status != StatusCode::Found {
        eprintln!("Could not find resource: {status}");
        return Err(Error::Status(status));
    }
    let json: Value = response.body_json().await.unwrap();
    let asset = json.key("assets")?.index(1)?;
    let url = asset.key("url")?.get_string()?;
    let size = asset.key("size")?.get_u64()?;
    Ok((url, size))
}

async fn download(url: &str, path: &Path, asset_size: u64, tx: Sender<f32>) -> Result<(), Error> {
    let request = surf::get(url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", USER_AGENT);
    let mut response = surf::client()
        .with(Redirect::default())
        .send(request)
        .await?;
    let status = response.status();
    if status != StatusCode::Ok && status != StatusCode::Found {
        eprintln!("Could not find resource.");
        return Err(Error::Status(status));
    }
    let mut downloaded: u64 = 0;
    let mut log = File::create(path.join("log.txt")).await?;
    let mut file = File::create(path.join("undertale_mod_manager.exe.download")).await?;
    let mut buffer = [0_u8; CHUNK_SIZE];
    loop {
        let bytes_len = response.read(&mut buffer).await?;
        if bytes_len == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_len]).await?;
        downloaded += bytes_len as u64;

        let percentage = downloaded as f32 / asset_size as f32;
        log.write_all(
            format!("Wrote {downloaded}/{asset_size} bytes. ({percentage}%)\n").as_bytes(),
        )
        .await?;

        if let Err(err) = tx.force_send(percentage) {
            eprintln!("Faied to send percentage: {err}");
        };
    }
    // println!("Bytes: {:?}", response.body_bytes().await?);
    // println!("String: {}", response.body_string().await?);
    // println!("Status: {}", response.status());
    // println!(
    //     "Headers: {:?}",
    //     response
    //         .header_names()
    //         .zip(response.header_values())
    //         .collect::<Vec<_>>()
    // );
    // file.write_all(&response.body_bytes().await?).await?;

    file.flush().await?;
    log.flush().await?;

    println!("Finished. ({downloaded} bytes).");

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
