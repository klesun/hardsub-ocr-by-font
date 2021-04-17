extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use std::fs::File;
use std::io::prelude::*;
use crate::ppm_format;

fn make_scaler(decoder: &ffmpeg::decoder::video::Video) -> Result<Context, ffmpeg::Error> {
    Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )
}

const PIXEL_COLORS: &[&str] = &["RED", "GREEN", "BLUE"];

/// should have taken from ffprobe, but nah...
const FRAME_RATE: f32 = 24.390244;

/// summed change of R,G,B values of a pixel to be considered
const PIXEL_NOISE_THRESHOLD: u32 = 20;

// Maa-chan's text block change: 0.014942350898686597
// auto-play indicator blink   : 0.00033933171069353485
//                               0.000327551969862017
// noise                       : 0.00004599985248729856
//                               0.0000008680555555555528
// image quality jump          : 0.0020935358228921473
//                               0.00655814412007137
//                               0.0057469362746679735
// game text animation frame   : 0.0011677531545020159
const CHANGE_FACTOR_THRESHOLD: f64 = 0.001;

/// lengthy text is usually about 50k pixels
const QUALITY_JUMP_POINTS_THRESHOLD: usize = 75000;

struct NewFrameInfo {
    change_factor: f64,
    real_points_changed: usize,
    is_text_change: bool,
    /// meaningful only when is_text_change set
    text_only_frame: Vec<u8>,
}

/// compares every pixel in both frames and returns a float number in range [0..1]
/// representing how much did the colors change (0 = completely same image,
/// 1 = completely white image changed to completely black or vice-versa)
fn analyze_new_frame(old_frame: &Video, new_frame: &Video) -> NewFrameInfo {
    // TODO: detect that considerable change is only applied to a _part_ of image, to
    let old_pixel_bytes = old_frame.data(0);
    let new_pixel_bytes = new_frame.data(0);
    if old_pixel_bytes.len() != new_pixel_bytes.len() {
        panic!(
            "pizdeeeeeeec, two frames have different bitmap sizes: {} and {}",
            old_pixel_bytes.len(),
            new_pixel_bytes.len()
        )
    }
    let bitmap_size = old_pixel_bytes.len() as usize;
    let width = old_frame.width();

    let mut real_points_changed = 0;
    let mut text_only_frame = vec![0; bitmap_size];
    let mut total_change: f64 = 0.0;

    for pixel_index in 0..bitmap_size / 3 {
        let mut pixel_change: u32 = 0;
        let x = (pixel_index % width as usize) as u32;
        let y = (pixel_index / width as usize) as u32;
        for color_index in 0..PIXEL_COLORS.len() as u8 {
            let byte_index = pixel_index * 3 + color_index as usize;
            // does not matter whether it's red, green or blue byte
            let old_byte = old_pixel_bytes[byte_index];
            let new_byte = new_pixel_bytes[byte_index];
            let change_byte = if old_byte > new_byte {
                old_byte - new_byte
            } else {
                new_byte - old_byte
            };
            if change_byte > 0 {
                let color_str = PIXEL_COLORS[color_index as usize];
                // println!("x: {}, y: {}, color: {}, change: {}", x, y, color_str, change_byte);
                total_change += change_byte as f64 / 255.0;
            }
            pixel_change += change_byte as u32;
        }
        if pixel_change >= PIXEL_NOISE_THRESHOLD {
            for color_index in 0..PIXEL_COLORS.len() as u8 {
                let byte_index = pixel_index * 3 + color_index as usize;
                text_only_frame[byte_index] = new_pixel_bytes[byte_index];
            }
            real_points_changed += 1;
        }
    }
    let change_factor = total_change / old_pixel_bytes.len() as f64;

    let is_text_change = change_factor > CHANGE_FACTOR_THRESHOLD
        && real_points_changed < QUALITY_JUMP_POINTS_THRESHOLD
        && real_points_changed > 15000;

    return NewFrameInfo {
        change_factor,
        real_points_changed,
        is_text_change,
        text_only_frame,
    };
}

fn save_file(
    bitmap: &[u8],
    ppm_header: &str,
    name: String,
) -> std::result::Result<(), std::io::Error> {
    let mut file = File::create(format!("out/change_frames/{}.ppm", name))?;
    file.write_all(ppm_header.as_bytes())?;
    file.write_all(bitmap)?;
    Ok(())
}

/// iterate through frames of a video file and, using few heuristic numbers,
/// detect frames in which hardsub text changes and dump these frames to file
pub fn detect_still_frames() -> Result<(), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let path = "assets/fmd_muramasa_maachan_05.webm";

    if let Ok(mut ictx) = input(&path) {
        let input = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let video_stream_index = input.index();

        let mut decoder = input.codec().decoder().video()?;
        let mut scaler = make_scaler(&decoder)?;

        let mut frame_index = 0;
        let mut last_frame = Video::empty();

        let mut receive_and_process_decoded_frames =
            |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                let mut decoded = Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    let mut rgb_frame = Video::empty();

                    scaler.run(&decoded, &mut rgb_frame)?;
                    let ppm_header = ppm_format::make_header(
                        rgb_frame.width() as usize,
                        rgb_frame.height() as usize
                    );

                    if frame_index > 0 {
                        let seconds = (frame_index as f32 / FRAME_RATE).floor();
                        let rel_frame_index = (frame_index as f32 % FRAME_RATE).round();
                        let info = analyze_new_frame(&last_frame, &rgb_frame);
                        if info.is_text_change {
                            println!(
                                "Frame {} at {}:{} s. change factor: {}, points: {}",
                                frame_index,
                                seconds,
                                rel_frame_index,
                                info.change_factor,
                                info.real_points_changed
                            );
                            save_file(
                                rgb_frame.data(0),
                                &ppm_header,
                                format!("frame{}_old", frame_index),
                            ).unwrap();

                            save_file(
                                &info.text_only_frame,
                                &ppm_header,
                                format!("frame{}_new", frame_index),
                            ).unwrap();
                        }
                    }
                    frame_index += 1;
                    last_frame = rgb_frame;
                }
                Ok(())
            };

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet)?;
                receive_and_process_decoded_frames(&mut decoder)?;
            }
        }
        decoder.send_eof()?;
        receive_and_process_decoded_frames(&mut decoder)?;
    }

    Ok(())
}
