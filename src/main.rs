
mod mongo;
mod msg_parser;
mod utils;

use std::path::Path;
use dotenv::dotenv;

use mongo::models::Video;
use mongodb::Collection;
use utils::{builder, verify, thumb_gen_builder};
use aws_sdk_sqs as sqs;
use mongo::helpers::update_video_status;




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


async fn load_env() -> (bool, aws_config::SdkConfig, String) {
    dotenv().expect("Failed to read .env file");
    let ffmpeg_log = std::env::var("FFMPEG_LOG")
        .map(|s| s == "1")
        .unwrap_or(false);

    let config = aws_config::load_from_env().await;
    let queue_url = std::env::var("QUEUE_URL").expect("QUEUE_URL not set");

    (ffmpeg_log, config, queue_url)
}

async fn process_messages(
    sqs_client: &sqs::Client,
    queue_url: &str,
    video_collection: &Collection<Video>,
    ffmpeg_log: bool,
) {
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
                let parsed = msg_parser::parse_msg(body);
                println!("{:?}", parsed);

                match parsed {
                    Ok(p) => {
                        let key = &p.records[0].s3.object.key;
                        let bucket = &p.records[0].s3.bucket.name;
                        println!("key: {}", key);
                        println!("bucket: {}", bucket);

                        let res = runner(key, video_collection, ffmpeg_log).await;

                        match res {
                            Ok(_) => {
                                println!("Success");
                            }
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
    let (ffmpeg_log, config, queue_url) = load_env().await;
    let sqs_client = sqs::Client::new(&config);
    let video_collection = mongo::get_video_collection().await.unwrap();
    process_messages(&sqs_client, &queue_url, &video_collection, ffmpeg_log).await;
}

pub struct FilePaths {
    pub input_file: String,
    pub output_dir: String,
    pub file_name_without_ext: String,
}

pub fn generate_paths(key: &str) -> Result<FilePaths, Box<dyn std::error::Error>> {
    let path = Path::new(S3_MOUNT_POINT).join(key);

    if path.exists() == false {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            std::fmt::format(format_args!("File not found: {:?}", path))
        )));
    }

    let input_file = path.to_str().ok_or("Failed to convert path to string")?;
    println!("input_file: {}", input_file);

    let file_name_without_ext = path
        .file_stem()
        .ok_or("Failed to get file stem")?
        .to_str()
        .ok_or("Failed to convert OsStr to string")?;

    // example: /home/rushikesh/s3_drive/processed/93f2b0b08fsdf8e9b0fsdf8bb
    let output_dir = Path::new(OUTPUT_DIR).join(file_name_without_ext);
    if !output_dir.exists() {
        println!("Creating output dir: {:?}", output_dir);
        std::fs::create_dir_all(&output_dir)?;
    } else {
        // println!("Output dir already exists: {:?}", output_dir);
        // throw error
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Output dir already exists",
        )));
    }

    let output_dir = output_dir
        .to_str()
        .ok_or("Failed to convert path to string")?;

    // let playlist_path = output_dir.join(MASTER_PLAYLIST);
    // let playlist_path = playlist_path
    //     .to_str()
    //     .ok_or("Failed to convert path to string")?;

    Ok(FilePaths {
        input_file: input_file.to_string(),
        output_dir: output_dir.to_string(),
        file_name_without_ext: file_name_without_ext.to_string(),
    })
}

pub async fn runner(
    key: &str,
    video_collection: &Collection<Video>,
    ffmpeg_log: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_paths = generate_paths(key)?;

    update_video_status(
        video_collection,
        &file_paths.file_name_without_ext,
        mongo::models::VideoStatus::PROCESSING,
    )
    .await?;

    transcoder(&file_paths, ffmpeg_log)?;

    update_video_status(
        video_collection,
        &file_paths.file_name_without_ext,
        mongo::models::VideoStatus::READY,
    )
    .await?;

    Ok(())
}

pub fn transcoder(
    file_paths: &FilePaths,
    ffmpeg_log: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream_info = verify::check_streams(&file_paths.input_file)?;

    println!("Stream info: {:?}", stream_info);


    // multiple quality transcoding
    let res_idx = builder::get_highest_possible_res_idx(&stream_info);

    let command = builder::build_transcoder_cmd(
        &file_paths.input_file,
        &file_paths.output_dir,
        &stream_info,
        res_idx,
    );

    // ! need to be called
    builder::run_ffmpeg(command, ffmpeg_log);


    // thumbnail generation
    let command = thumb_gen_builder::build_thumb_cmd(
        &file_paths.input_file,
        &file_paths.output_dir,
        &stream_info,
    );


    // ! need to be called
    builder::run_ffmpeg(command, ffmpeg_log);

    Ok(())
}
