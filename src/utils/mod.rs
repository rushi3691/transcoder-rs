pub mod builder;
pub mod verify;
pub mod thumb_gen_builder;

#[derive(Debug)]
/// The `StreamInfo` struct contains information about a video stream, including whether it has video
/// and audio, its dimensions, bitrate, and key frames interval.
///
/// Properties:
///
/// * `has_video`: A boolean value indicating whether the input file has a video stream or not. If it is
/// true, it means the input file has a video stream. If it is false, it means the input file does not
/// have a video stream.
/// * `has_audio`: The `has_audio` property indicates whether the input file has an audio stream. If it
/// is `true`, it means the input file has an audio stream. If it is `false`, it means the input file
/// does not have an audio stream.
/// * `width`: The width property represents the width of the video stream in pixels. It indicates the
/// horizontal size of the video frame.
/// * `height`: The `height` property represents the height of the video stream in pixels. It indicates
/// the vertical size of the video frame.
/// * `bitrate`: The `bitrate` property represents the rate at which bits are transmitted in the video
/// stream. It is typically measured in kilobits per second (kbps) or megabits per second (Mbps). The
/// bitrate determines the quality and size of the video file, with higher bitrates resulting in better quality
/// * `key_frames_interval`: The `key_frames_interval` property represents the interval between key
/// frames in the video stream. Key frames are frames that are complete and self-contained, meaning they
/// can be decoded without relying on any other frames. They are important for video compression and
/// seeking within a video.
pub struct StreamInfo {
    pub has_video: bool,
    pub has_audio: bool,
    pub width: u32,
    pub height: u32,
    pub bitrate: usize,
    pub key_frames_interval: i32,
    pub total_frames: f64,
    pub duration: f64,
}
