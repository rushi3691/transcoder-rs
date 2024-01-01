extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::input;
use ffmpeg::media;
use ffmpeg_next::Error;

use super::StreamInfo;


pub fn check_streams(input_file: &str) -> Result<StreamInfo, Error> {
    let ictx = input(&input_file).unwrap();

    let mut stream_info = StreamInfo {
        has_audio: true,
        has_video: true,
        width: 0,
        height: 0,
        bitrate: 0,
        key_frames_interval: 0,
    };
    let input_video_stream = ictx.streams().best(media::Type::Video).unwrap_or_else(|| {
        eprintln!("Input file does not contain a video stream");
        panic!("Input file does not contain a video stream")
    });
    let input_audio_stream = ictx.streams().best(media::Type::Audio);

    if input_audio_stream.is_none() {
        eprintln!("Input file does not contain an audio stream");
        stream_info.has_audio = false;
    }

    let input_video_stream = input_video_stream.codec().decoder().video().unwrap();
    let width = input_video_stream.width();
    let height = input_video_stream.height();
    let bitrate = input_video_stream.bit_rate();
    // let duration = ictx.duration() / ffmpeg::ffi::AV_TIME_BASE as i64; // in seconds
    // key_frames_interval="$(echo `ffprobe ${source} 2>&1 | grep -oE '[[:digit:]]+(.[[:digit:]]+)? fps' | grep -oE '[[:digit:]]+(.[[:digit:]]+)?'`*2 | bc || echo '')"
    // key_frames_interval=${key_frames_interval:-50}
    // key_frames_interval=$(echo `printf "%.1f\n" $(bc -l <<<"$key_frames_interval/10")`*10 | bc) # round
    // key_frames_interval=${key_frames_interval%.*} # truncate to integer
    let frame_rate: ffmpeg::Rational = input_video_stream.frame_rate().unwrap();
    let frame_rate = frame_rate.numerator() as f64 / frame_rate.denominator() as f64;
    let key_frames_interval = (frame_rate * 2.0).round() as i32;
    let key_frames_interval = (key_frames_interval / 10) as f64 * 10.0;
    let key_frames_interval = key_frames_interval.round() as i32;

    // let total_frames = duration as f64 * frame_rate;

    stream_info.key_frames_interval = key_frames_interval;
    stream_info.width = width;
    stream_info.height = height;
    stream_info.bitrate = bitrate;

    return Ok(stream_info);
}
