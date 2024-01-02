use crate::POSSIBLE_RESOLUTIONS;
use super::StreamInfo;

//     ffmpeg -i $input_file \
// -vf "select=eq(n\,$middle_frame)" -vframes 1 -s 3840x2160 -map 0:v $infput_file_4k.jpg \
// -vf "select=eq(n\,$middle_frame)" -vframes 1 -s 1920x1080 -map 0:v $infput_file_1080p.jpg

pub fn build_thumb_cmd(
    input_file: &str,
    output_dir: &str,
    stream_info: &StreamInfo,
) -> Vec<String> {
    let mut command: Vec<String> = vec!["-i".to_string(), input_file.to_string()];

    let total_frames = stream_info.total_frames;
    let middle_frame = (total_frames / 2.0).round() as i32;

    command.push("-vf".to_string());
    command.push(format!("select=eq(n\\,{})", middle_frame));
    command.push("-vframes".to_string());
    command.push("1".to_string());
    command.push("-s".to_string());
    command.push("3840x2160".to_string());
    command.push("-map".to_string());
    command.push("0:v".to_string());
    command.push(format!("{}/thumb_4k.jpg", output_dir));


    command.push("-vf".to_string());
    command.push(format!("select=eq(n\\,{})", middle_frame));
    command.push("-vframes".to_string());
    command.push("1".to_string());
    command.push("-s".to_string());
    command.push("1920x1080".to_string());
    command.push("-map".to_string());
    command.push("0:v".to_string());
    command.push(format!("{}/thumb_1080p.jpg", output_dir));

    return command;
}