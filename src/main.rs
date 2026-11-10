
mod amqp;
mod cloudflare;
mod mongo;
mod msg_parser;
mod utils;

use std::path::Path;
use dotenv::dotenv;

use mongo::models::Video;
use mongodb::Collection;
use utils::{builder, verify, thumb_gen_builder, sprite};
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

// legacy AWS: S3 bucket mounted with s3fs
const S3_MOUNT_POINT: &str = "/home/rushikesh/s3_drive";


fn ffmpeg_log_enabled() -> bool {
    std::env::var("FFMPEG_LOG")
        .map(|s| s == "1")
        .unwrap_or(false)
}

/// Local scratch space for downloads and transcoder output before upload to R2.
fn work_dir() -> std::path::PathBuf {
    std::env::var("WORK_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("streamcraft"))
}

// legacy AWS: SQS consumer config
#[allow(dead_code)]
async fn load_env() -> (bool, aws_config::SdkConfig, String) {
    dotenv().expect("Failed to read .env file");
    let ffmpeg_log = ffmpeg_log_enabled();

    let config = aws_config::load_from_env().await;
    let queue_url = std::env::var("QUEUE_URL").expect("QUEUE_URL not set");

    (ffmpeg_log, config, queue_url)
}

// legacy AWS: SQS consumer
#[allow(dead_code)]
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

/// Run one job message (R2 event shape) end to end.
async fn handle_job(
    data: &[u8],
    r2: &cloudflare::r2::R2,
    video_collection: &Collection<Video>,
    ffmpeg_log: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let event = msg_parser::parse_r2_event(serde_json::from_slice(data)?)?;
    // the worker uploads processed/* to the same bucket; never re-process those
    if !event.object.key.starts_with("raw/") {
        println!("skipping {} {}", event.action, event.object.key);
        return Ok(());
    }
    println!("{} {}/{}", event.action, event.bucket, event.object.key);
    runner_r2(r2, &event.bucket, &event.object.key, video_collection, ffmpeg_log).await
}

/// CloudAMQP consumer. Acks after the transcode finishes, so a crash mid-job redelivers it.
// ponytail: one retry via `redelivered`, no dead-letter queue. Add a DLX if failed jobs need inspecting.
async fn process_amqp_jobs(
    amqp_url: &str,
    r2: &cloudflare::r2::R2,
    video_collection: &Collection<Video>,
    ffmpeg_log: bool,
) {
    use futures_lite::StreamExt;
    use lapin::options::{BasicAckOptions, BasicNackOptions};

    loop {
        let (_conn, mut consumer) = match amqp::consume(amqp_url).await {
            Ok(c) => c,
            Err(e) => {
                println!("AMQP connect error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };
        println!("listening for jobs on AMQP queue {}", amqp::QUEUE);

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    println!("AMQP delivery error: {}", e);
                    break;
                }
            };

            let settled = match handle_job(&delivery.data, r2, video_collection, ffmpeg_log).await {
                Ok(_) => {
                    println!("Success");
                    delivery.ack(BasicAckOptions::default()).await
                }
                Err(e) => {
                    // retry once, then drop (the video is already marked FAILED)
                    println!("Error (redelivered={}): {}", delivery.redelivered, e);
                    delivery
                        .nack(BasicNackOptions { requeue: !delivery.redelivered, ..Default::default() })
                        .await
                }
            };
            if let Err(e) = settled {
                println!("AMQP ack error: {}", e);
            }
        }

        println!("AMQP consumer stopped, reconnecting");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

#[::tokio::main]
async fn main() {
    dotenv().expect("Failed to read .env file");
    let ffmpeg_log = ffmpeg_log_enabled();
    let video_collection = mongo::get_video_collection().await.unwrap();

    // legacy AWS:
    // let (ffmpeg_log, config, queue_url) = load_env().await;
    // process_messages(&sqs::Client::new(&config), &queue_url, &video_collection, ffmpeg_log).await;

    let amqp_url = std::env::var("AMQP_URL").expect("AMQP_URL not set");
    let r2 = cloudflare::r2::R2::from_env();
    process_amqp_jobs(&amqp_url, &r2, &video_collection, ffmpeg_log).await;
}

pub struct FilePaths {
    pub input_file: String,
    pub output_dir: String,
    pub file_name_without_ext: String,
}

/// `root` holds `raw/<id>` inputs; outputs go to `root/processed/<id>`.
pub fn generate_paths(root: &Path, key: &str) -> Result<FilePaths, Box<dyn std::error::Error>> {
    let path = root.join(key);

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
    let output_dir = root.join("processed").join(file_name_without_ext);
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

// legacy AWS: reads and writes the s3fs mount directly
#[allow(dead_code)]
pub async fn runner(
    key: &str,
    video_collection: &Collection<Video>,
    ffmpeg_log: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_paths = generate_paths(Path::new(S3_MOUNT_POINT), key)?;

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

/// Download `key` from R2, transcode locally, upload outputs to `processed/<id>/`.
pub async fn runner_r2(
    r2: &cloudflare::r2::R2,
    bucket: &str,
    key: &str,
    video_collection: &Collection<Video>,
    ffmpeg_log: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = Path::new(key)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Failed to get file stem")?;
    let job_root = work_dir().join(id);
    // clear leftovers from a crashed attempt; generate_paths refuses an existing output dir
    let _ = std::fs::remove_dir_all(&job_root);

    let result = async {
        r2.download(bucket, key, &job_root.join(key)).await?;
        let file_paths = generate_paths(&job_root, key)?;

        update_video_status(video_collection, id, mongo::models::VideoStatus::PROCESSING).await?;
        transcoder(&file_paths, ffmpeg_log)?;
        r2.upload_dir(bucket, Path::new(&file_paths.output_dir), &format!("processed/{}", id))
            .await?;
        update_video_status(video_collection, id, mongo::models::VideoStatus::READY).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    let _ = std::fs::remove_dir_all(&job_root);
    if result.is_err() {
        let _ = update_video_status(video_collection, id, mongo::models::VideoStatus::FAILED).await;
    }
    result
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

    builder::run_ffmpeg(command, ffmpeg_log)?;


    // thumbnail generation
    let command = thumb_gen_builder::build_thumb_cmd(
        &file_paths.input_file,
        &file_paths.output_dir,
        &stream_info,
    );


    builder::run_ffmpeg(command, ffmpeg_log)?;

    // seek-bar preview sprite + its WebVTT index
    if let Some(plan) = sprite::plan(&stream_info) {
        let command = sprite::build_sprite_cmd(&file_paths.input_file, &file_paths.output_dir, &plan);
        builder::run_ffmpeg(command, ffmpeg_log)?;
        std::fs::write(
            Path::new(&file_paths.output_dir).join(sprite::VTT_FILE),
            sprite::build_vtt(&plan, stream_info.duration),
        )?;
    }

    Ok(())
}
