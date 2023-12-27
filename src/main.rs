// ffmpeg -i "c:/videos/sample.mp4
// -map 0:v:0 -map 0:a:0 -map 0:v:0 -map 0:a:0 -map 0:v:0 -map 0:a:0
// -c:v libx264 -crf 22 -c:a aac -ar 48000
// -filter:v:0 scale=w=480:h=360  -maxrate:v:0 600k -b:a:0 64k
// -filter:v:1 scale=w=640:h=480  -maxrate:v:1 900k -b:a:1 128k
// -filter:v:2 scale=w=1280:h=720 -maxrate:v:2 900k -b:a:2 128k
// -var_stream_map "v:0,a:0,name:360p v:1,a:1,name:480p v:2,a:2,name:720p"
// -preset slow -hls_list_size 0 -threads 0 -f hls -hls_playlist_type event -hls_time 3
// -hls_flags independent_segments -master_pl_name "name-pl.m3u8"
// "c:/videos/encoded/name-%v.m3u8"

use std::path::Path;
mod parser;
mod utils;
// use s3::presigning::PresigningConfig;
use utils::{builder, verify};
// use std::time::Duration;

use dotenv::dotenv;

// use aws_sdk_s3 as s3;
use aws_sdk_sqs as sqs;

const MASTER_PLAYLIST: &str = "name-%v.m3u8";
const POSSIBLE_RESOLUTIONS: [(u32, u32, u32, u32); 6] = [
    // width, height, bitrate, audio_bitrate
    (256, 144, 600, 64),
    (426, 240, 600, 64),
    (640, 360, 600, 64),
    (854, 480, 900, 128),
    (1280, 720, 900, 128),
    (1920, 1080, 900, 192),
];

const S3_MOUNT_POINT: &str = "/home/rushikesh/s3_drive";
const OUTPUT_DIR: &str = "/home/rushikesh/s3_drive/processed";

// const DOWNLOAD_URL_EXPIRES_IN: Duration = Duration::from_secs(60 * 60 * 24 * 7); // 7 days

async fn load_env() -> (bool, aws_config::SdkConfig, String) {
    dotenv().expect("Failed to read .env file");
    let ffmpeg_log = std::env::var("FFMPEG_LOG")
        .map(|s| s == "1")
        .unwrap_or(false);

    let config = aws_config::load_from_env().await;
    let queue_url = std::env::var("QUEUE_URL").expect("QUEUE_URL not set");

    (ffmpeg_log, config, queue_url)
}

async fn process_messages(sqs_client: &sqs::Client, queue_url: &str, ffmpeg_log: bool) {
    println!("listening for messages");
    loop {
        let request = sqs_client
            .receive_message()
            .set_wait_time_seconds(Some(20))
            .queue_url(queue_url)
            .send()
            .await;

        let request = match request {
            Err(e) => {
                println!("50: Error: {}", e);
                continue;
            }
            Ok(r) => r,
        };

        if let Some(messages) = request.messages {
            for message in messages {
                let body = message.body.as_ref().unwrap();
                let parsed = parser::parse_msg(body);
                println!("{:?}", parsed);

                match parsed {
                    Ok(p) => {
                        let key = &p.records[0].s3.object.key;
                        let bucket = &p.records[0].s3.bucket.name;
                        println!("key: {}", key);
                        println!("bucket: {}", bucket);

                        let r = transcoder(ffmpeg_log, key);
                        match r {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("91: Error: {}", e);
                    }
                }

                let x = sqs_client
                    .delete_message()
                    .queue_url(queue_url)
                    .receipt_handle(message.receipt_handle.unwrap())
                    .send()
                    .await;

                match x {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
        }
    }
}

#[::tokio::main]
async fn main() {
    // load .env file
    // dotenv().expect("Failed to read .env file");
    // let ffmpeg_log = std::env::var("FFMPEG_LOG")
    //     .map(|s| s == "1")
    //     .unwrap_or(false);

    // println!("FFMPEG_LOG: {}", ffmpeg_log);

    // let config = aws_config::load_from_env().await;

    // let queue_url = std::env::var("QUEUE_URL").expect("QUEUE_URL not set");

    // let s3_client = s3::Client::new(&config);

    // let download_url_config = PresigningConfig::expires_in(DOWNLOAD_URL_EXPIRES_IN);
    // let download_url_config = match download_url_config {
    //     Ok(c) => c,
    //     Err(e) => {
    //         println!("Error: {}", e);
    //         process::exit(1);
    //     }
    // };

    let (ffmpeg_log, config, queue_url) = load_env().await;
    let sqs_client = sqs::Client::new(&config);
    process_messages(&sqs_client, &queue_url, ffmpeg_log).await;

    // transcoder_test().unwrap();

    // generate hello world and write file to ~/s3-drive/processed/processed.txt

    // let path = Path::new(S3_MOUNT_POINT).join("processed/");
    // // let output_file = path.to_str().unwrap();

    // let output_folder = path.to_str().unwrap();

    // println!("output_folder: {}", output_folder);

    // if path.exists() == false {
    //     println!("path does not exist");
    //     // std::fs::create_dir_all(&output_folder).unwrap();
    // }

    // let output_file = Path::new(&output_folder).join("processed.txt");

    // let mut file = std::fs::File::create(output_file).unwrap();
    // file.write_all(b"Hello, world!").unwrap();

    // file.sync_all().unwrap();
}

pub fn transcoder_test() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().expect("Failed to read .env file");
    let ffmpeg_log = std::env::var("FFMPEG_LOG")
        .map(|s| s == "1")
        .unwrap_or(false);

    println!("FFMPEG_LOG: {}", ffmpeg_log);

    let key = "raw/sample.mp4";

    transcoder(ffmpeg_log, key)?;

    Ok(())
}

pub fn transcoder(ffmpeg_log: bool, key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(S3_MOUNT_POINT).join(key);
    let input_file = path.to_str().unwrap();

    println!("input_file: {}", input_file);
    let path = Path::new(&input_file);
    if path.exists() == false {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        )));
    }

    // if path.try_exists()? == false {
    //     return Err(Box::new(std::io::Error::new(
    //         std::io::ErrorKind::NotFound,
    //         "File not found",
    //     )));
    // }

    let file_name_without_ext = path
        .file_stem()
        .ok_or("Failed to get file stem")?
        .to_str()
        .ok_or("Failed to convert OsStr to string")?;
    let output_dir = Path::new(OUTPUT_DIR).join(file_name_without_ext);
    if !output_dir.exists() {
        println!("Creating output dir: {:?}", output_dir);
        std::fs::create_dir_all(&output_dir)?;
    }

    let binding = output_dir.join(MASTER_PLAYLIST);
    let playlist_path = binding.to_str().ok_or("Failed to convert path to string")?;

    let stream_info = verify::check_streams(&input_file)?;

    println!("Stream info: {:?}", stream_info);

    let res_idx = builder::get_highest_possible_res_idx(&stream_info);

    let command = builder::build_cmd(&input_file, &playlist_path, &stream_info, res_idx);

    // ! need to be called
    builder::run_ffmpeg(command, ffmpeg_log);

    Ok(())
}
