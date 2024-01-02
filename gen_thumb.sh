# ffmpeg -i sample.mp4 \
# -vf "select='not(mod(n\,round(fps*duration/2)))'" -vframes 1 -s 3840x2160 -map 0:v thumbnail_4k.jpg \
# -vf "select='not(mod(n\,round(fps*duration/2)))'" -vframes 1 -s 1920x1080 -map 0:v thumbnail_1080p.jpg


#!/bin/bash

input_file="sample.mp4"
# Get the total number of frames in the video
total_frames=$(ffprobe -v error -select_streams v:0 -show_entries stream=nb_frames -of default=nokey=1:noprint_wrappers=1 $input_file)

# Calculate the middle frame
middle_frame=$((total_frames / 2))

# # Generate the 4K thumbnail
# ffmpeg -i $input_file -vf "select=eq(n\,$middle_frame)" -vframes 1 -s 3840x2160 thumbnail_4k.jpg

# # Generate the 1080p thumbnail
# ffmpeg -i $input_file -vf "select=eq(n\,$middle_frame)" -vframes 1 -s 1920x1080 thumbnail_1080p.jpg


ffmpeg -i $input_file \
-vf "select=eq(n\,$middle_frame)" -vframes 1 -s 3840x2160 -map 0:v thumbnail_4k.jpg \
-vf "select=eq(n\,$middle_frame)" -vframes 1 -s 1920x1080 -map 0:v thumbnail_1080p.jpg