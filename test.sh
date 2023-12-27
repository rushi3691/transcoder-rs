# first command
ffmpeg -i "sample.mp4" \
-map 0:v:0 -map 0:a:0 -map 0:v:0 -map 0:a:0 -map 0:v:0 -map 0:a:0 \
-c:v libx264 -crf 22 -c:a aac -ar 48000 \
-filter:v:0 scale=w=480:h=360  -maxrate:v:0 600k -b:a:0 64k \
-filter:v:1 scale=w=640:h=480  -maxrate:v:1 900k -b:a:1 128k \
-filter:v:2 scale=w=1280:h=720 -maxrate:v:2 900k -b:a:2 128k \
-var_stream_map "v:0,a:0,name:360p v:1,a:1,name:480p v:2,a:2,name:720p" \
-preset slow -hls_list_size 0 -threads 0 -f hls -hls_playlist_type vod -hls_time 2 \
-hls_flags independent_segments -hls_segment_filename "static/encoded/%v/name_%03d.ts" -master_pl_name "master.m3u8" "static/encoded/%v/sub-pl.m3u8" #"static/encoded/name-%v.m3u8" -master_pl_name "name-pl.m3u8"

# second command
ffmpeg -hide_banner -y -i sample.mp4 \
-c:a aac -ar 48000 -c:v h264 -profile:v main -crf 19 \
-sc_threshold 0 -g 50 -keyint_min 50 \
-hls_time 10 -hls_playlist_type vod \
-vf scale=w=-2:h=240 -b:v 400k -maxrate 428k -bufsize 600k -b:a 128k \
-hls_segment_filename output/240p_%03d.ts output/240p.m3u8 \
-vf scale=w=-2:h=360 -b:v 800k -maxrate 856k -bufsize 1200k -b:a 128k \
-hls_segment_filename output/360p_%03d.ts output/360p.m3u8 \
-vf scale=w=-2:h=480 -b:v 1400k -maxrate 1498k -bufsize 2100k -b:a 192k \
-hls_segment_filename output/480p_%03d.ts output/480p.m3u8 \
-vf scale=w=-2:h=720 -b:v 2800k -maxrate 2996k -bufsize 4200k -b:a 192k \
-hls_segment_filename output/720p_%03d.ts output/720p.m3u8 \
-vf scale=w=-2:h=1080 -b:v 5000k -maxrate 5350k -bufsize 7500k -b:a 256k \
-hls_segment_filename output/1080p_%03d.ts output/1080p.m3u8