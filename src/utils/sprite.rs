//! Seek-bar preview sprites: one tiled JPEG plus a WebVTT file mapping time ranges to tiles.

use super::StreamInfo;

const TILE_WIDTH: u32 = 160;
const COLUMNS: u32 = 10;
// ponytail: one sheet capped at ~200 tiles (1600x1800 for 16:9), so long videos get coarser
// previews (1h -> one every 18s). Split into several sheets if finer previews matter there.
const MAX_TILES: f64 = 200.0;

pub const SPRITE_FILE: &str = "sprite.jpg";
pub const VTT_FILE: &str = "sprite.vtt";

#[derive(Debug, PartialEq)]
pub struct Sprite {
    /// seconds between tiles
    pub interval: u32,
    pub count: u32,
    pub tile_width: u32,
    pub tile_height: u32,
}

pub fn plan(stream_info: &StreamInfo) -> Option<Sprite> {
    if stream_info.duration <= 0.0 || stream_info.width == 0 || stream_info.height == 0 {
        return None;
    }
    let interval = (stream_info.duration / MAX_TILES).ceil().max(1.0) as u32;
    let count = ((stream_info.duration / interval as f64).ceil() as u32).max(1);
    // keep the source aspect ratio; the scaler needs an even height
    let tile_height =
        (TILE_WIDTH as f64 * stream_info.height as f64 / stream_info.width as f64 / 2.0).round() as u32 * 2;
    Some(Sprite { interval, count, tile_width: TILE_WIDTH, tile_height })
}

/// ffmpeg -i in -vf "fps=1/N,scale=W:H,tile=10xR" -frames:v 1 -an out/sprite.jpg
pub fn build_sprite_cmd(input_file: &str, output_dir: &str, sprite: &Sprite) -> Vec<String> {
    let rows = sprite.count.div_ceil(COLUMNS);
    vec![
        "-i".to_string(),
        input_file.to_string(),
        "-vf".to_string(),
        format!(
            "fps=1/{},scale={}:{},tile={}x{}",
            sprite.interval, sprite.tile_width, sprite.tile_height, COLUMNS, rows
        ),
        "-frames:v".to_string(),
        "1".to_string(),
        "-an".to_string(),
        format!("{}/{}", output_dir, SPRITE_FILE),
    ]
}

/// Cue k covers [k*interval, (k+1)*interval) and points at tile k (row-major).
/// The image URL is relative, so the player resolves it next to the .vtt file.
pub fn build_vtt(sprite: &Sprite, duration: f64) -> String {
    let mut vtt = String::from("WEBVTT\n");
    for k in 0..sprite.count {
        let start = (k * sprite.interval) as f64;
        let end = ((k + 1) * sprite.interval) as f64;
        let x = (k % COLUMNS) * sprite.tile_width;
        let y = (k / COLUMNS) * sprite.tile_height;
        vtt.push_str(&format!(
            "\n{} --> {}\n{}#xywh={},{},{},{}\n",
            timestamp(start),
            timestamp(end.min(duration)),
            SPRITE_FILE,
            x,
            y,
            sprite.tile_width,
            sprite.tile_height
        ));
    }
    vtt
}

fn timestamp(secs: f64) -> String {
    let ms = (secs * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(width: u32, height: u32, duration: f64) -> StreamInfo {
        StreamInfo {
            has_video: true,
            has_audio: true,
            width,
            height,
            bitrate: 0,
            key_frames_interval: 0,
            total_frames: 0.0,
            duration,
        }
    }

    #[test]
    fn plans_interval_count_and_aspect() {
        // 30s 480x270 (the "xyz" upload): one tile per second, 16:9 tiles
        assert_eq!(
            plan(&info(480, 270, 30.0)),
            Some(Sprite { interval: 1, count: 30, tile_width: 160, tile_height: 90 })
        );
        // 1h: capped at 200 tiles -> every 18s
        let long = plan(&info(1920, 1080, 3600.0)).unwrap();
        assert_eq!((long.interval, long.count), (18, 200));
        // vertical video keeps its shape, even height
        assert_eq!(plan(&info(1080, 1920, 10.0)).unwrap().tile_height, 284);
        // unknown duration -> no sprite
        assert_eq!(plan(&info(1920, 1080, 0.0)), None);
    }

    #[test]
    fn vtt_maps_cues_to_tiles() {
        let sprite = Sprite { interval: 1, count: 12, tile_width: 160, tile_height: 90 };
        let vtt = build_vtt(&sprite, 11.5);
        assert!(vtt.starts_with("WEBVTT\n"));
        assert_eq!(vtt.matches(" --> ").count(), 12);
        assert!(vtt.contains("\n00:00:00.000 --> 00:00:01.000\nsprite.jpg#xywh=0,0,160,90\n"));
        // 11th tile wraps to the second row
        assert!(vtt.contains("\n00:00:10.000 --> 00:00:11.000\nsprite.jpg#xywh=0,90,160,90\n"));
        // last cue ends at the video's duration
        assert!(vtt.contains("\n00:00:11.000 --> 00:00:11.500\nsprite.jpg#xywh=160,90,160,90\n"));
        assert_eq!(timestamp(3725.25), "01:02:05.250");
    }
}
