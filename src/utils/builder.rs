use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use crate::POSSIBLE_RESOLUTIONS;

use super::StreamInfo;

// ffmpeg -i "static/test.mp4" \
// -map 0:v:0 -map 0:a:0 -map 0:v:0 -map 0:a:0 -map 0:v:0 -map 0:a:0 \
// -c:v libx264 -crf 22 -c:a aac -ar 48000 \
// -filter:v:0 scale=w=480:h=360  -maxrate:v:0 600k -b:a:0 64k \
// -filter:v:1 scale=w=640:h=480  -maxrate:v:1 900k -b:a:1 128k \
// -filter:v:2 scale=w=1280:h=720 -maxrate:v:2 900k -b:a:2 128k \
// -var_stream_map "v:0,a:0,name:360p v:1,a:1,name:480p v:2,a:2,name:720p" \
// -preset slow -hls_list_size 0 -threads 0 -f hls -hls_playlist_type event -hls_time 2 \
// -hls_flags independent_segments
// -hls_segment_filename "output_dir/%v/seg_%03d.ts"
// -master_pl_name "master.m3u8"
// "output_dir/%v/sub-pl.m3u8"

// const HLS_TIME: u32 = 5;

pub fn get_highest_possible_res_idx(stream_info: &StreamInfo) -> usize {
    let mut maximum_resolution_at_idx: usize = 0;
    for (idx, (_, h, _, _)) in POSSIBLE_RESOLUTIONS.iter().enumerate() {
        if *h <= stream_info.height {
            maximum_resolution_at_idx = idx;
        } else {
            break;
        }
    }

    println!(
        "Max possible resolution: {}x{}",
        POSSIBLE_RESOLUTIONS[maximum_resolution_at_idx].0,
        POSSIBLE_RESOLUTIONS[maximum_resolution_at_idx].1
    );

    return maximum_resolution_at_idx;
}

pub fn build_transcoder_cmd(
    input_file: &str,
    output_dir: &str,
    stream_info: &StreamInfo,
    res_idx: usize,
) -> Vec<String> {
    let mut command: Vec<String> = vec!["-i".to_string(), input_file.to_string()];

    // add video and audio maps
    for _ in 0..=res_idx {
        command.push("-map".to_string());
        command.push("0:v:0".to_string());
        if stream_info.has_audio {
            command.push("-map".to_string());
            command.push("0:a:0".to_string());
        }
    }

    // add video and audio codecs
    command.push("-c:v".to_string());
    command.push("libx264".to_string());
    command.push("-crf".to_string());
    command.push("22".to_string());
    command.push("-c:a".to_string());
    command.push("aac".to_string());
    command.push("-ar".to_string());
    command.push("48000".to_string());

    // add filters
    for (idx, (w, h, vbr, abr)) in POSSIBLE_RESOLUTIONS.iter().enumerate() {
        if idx > res_idx {
            break;
        }
        command.push("-filter:v:".to_string() + &idx.to_string());
        command.push("scale=w=".to_string() + &w.to_string() + ":h=" + &h.to_string());
        command.push("-maxrate:v:".to_string() + &idx.to_string());
        command.push(vbr.to_string() + "k");

        if stream_info.has_audio {
            command.push("-b:a:".to_string() + &idx.to_string());
            command.push(abr.to_string() + "k");
        }
    }

    // add var_stream_map
    command.push("-var_stream_map".to_string());
    let mut var_stream_map = String::new();
    for (idx, (_, h, _, _)) in POSSIBLE_RESOLUTIONS.iter().enumerate() {
        if idx > res_idx {
            break;
        }
        if stream_info.has_audio {
            var_stream_map.push_str(format!("v:{},a:{},name:{}p ", idx, idx, h).as_str());
        } else {
            var_stream_map.push_str(format!("v:{},name:{}p ", idx, h).as_str());
        }
    }
    var_stream_map.pop(); // remove last space
                          // var_stream_map = format!("\"{}\"", var_stream_map);
    command.push(var_stream_map);

    // add preset
    command.push("-preset".to_string());
    // command.push("slow".to_string());
    command.push("fast".to_string());

    // add hls_list_size
    command.push("-hls_list_size".to_string());
    command.push("0".to_string());

    // add threads
    command.push("-threads".to_string());
    command.push("0".to_string());

    // add f
    command.push("-f".to_string());
    command.push("hls".to_string());

    // add hls_playlist_type
    command.push("-hls_playlist_type".to_string());
    // command.push("event".to_string());
    command.push("vod".to_string());

    // add hls_time
    // command.push("-hls_time".to_string());
    // command.push(HLS_TIME.to_string());

    // add hls_flags
    command.push("-hls_flags".to_string());
    command.push("independent_segments".to_string());

    // add output path
    // command.push("-master_pl_name".to_string());
    // command.push("master.m3u8".to_string());

    // add name%v.m3u8
    // command.push(output_path_m3u8.to_string());

    command.push("-hls_segment_filename".to_string());
    command.push(output_dir.to_string() + "/%v/seg_%03d.ts");

    command.push("-master_pl_name".to_string());
    command.push("master.m3u8".to_string());

    command.push(output_dir.to_string() + "/%v/sub-pl.m3u8");

    return command;
}

pub fn run_ffmpeg(command: Vec<String>, output_logs: bool) {
    println!("Running command: ffmpeg {:?}", command.join(" "));

    if output_logs {
        let mut binding = Command::new("ffmpeg");
        let command = binding.args(&command);
        command.stdout(Stdio::piped());

        let mut child = command.spawn().expect("failed to execute process");

        if let Some(ref mut stdout) = child.stdout {
            let reader = BufReader::new(stdout);

            for line in reader.lines() {
                println!("{}", line.unwrap());
            }
        }

        let res = child.wait();

        match res {
            Ok(status) => println!("Exited with status: {}", status.success()),
            Err(err) => println!("Failed to wait for child: {}", err),
        }
    } else {
        let output = Command::new("ffmpeg").args(&command).output();

        match output {
            Ok(output) => println!("Output: {:?}", output.status),
            Err(err) => println!("Failed to wait for child: {}", err),
        }
    }
}
