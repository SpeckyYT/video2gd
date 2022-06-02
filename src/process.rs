use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;

use image::*;

const COLOR_PRECISION: u8 = 4;

pub fn manage_process_frames(frames_folder: &PathBuf, video_files: &Vec<PathBuf>) -> Vec<Vec<HashMap<(usize,usize), [u8; 4]>>> {
    let mut videos = Vec::new();
    for video_file in video_files {
        videos.push(process_frames(&frames_folder, &video_file));
    }
    videos
}

fn process_frames(frames_folder: &PathBuf, file_path: &PathBuf) -> Vec<HashMap<(usize,usize), [u8; 4]>> {
    let file_name = file_path.file_stem().unwrap().to_str().unwrap();
    let current_folder = frames_folder.join(file_name);

    let frames: Vec<_> = current_folder.read_dir().expect("Failed reading frames directory at process_frames").collect();
    let frames: Vec<_> = frames.iter().map(|f|
        if f.is_err() {
            f.as_ref().map_err(|err| format!("Frame reading error: {}", err)).unwrap()
        } else {
            f.as_ref().unwrap()
        }
    ).collect();

    let frame_count = frames.len();

    let first_frame = image::open(frames[0].path()).unwrap();

    let (o_width, o_height) = first_frame.dimensions();
    let o_pixels: f64 = (o_width as f64) * (o_height as f64);
    
    let ratio: f64 = (o_width as f64) / (o_height as f64);
    let scale: f64 = (o_pixels / 999_f64).sqrt();
    
    let width = (ratio * (o_width as f64) / scale) as usize;
    let height = ((o_height as f64) / scale) as usize;

    let mut processes = Vec::new();

    for frame_index in 0..frame_count {
        let frame_entry = frames[frame_index].path();
        let thread = thread::spawn(move || {
            let frame_path = frame_entry;
            let frame = image::open(frame_path).expect("Failed to read frame");
            let resized = frame.resize(width as u32, height as u32, imageops::FilterType::Nearest);
            let pixels = resized.as_rgba8().expect("Wasn't able to get rgba8 from image").pixels();

            let mut output: Vec<[u8; 4]> = Vec::new();
            for pixel in pixels {
                output.push(flatten_color(&pixel.0, COLOR_PRECISION));
            }
            return output;
        });
        processes.push(thread);
        log("Frames spawned", processes.len(), frame_count)
    }

    let mut frames = Vec::new();
    let frame_count = processes.len();
    for process in processes {
        frames.push(process.join().unwrap());
        log("Frames resized", frames.len(), frame_count)
    }

    let mut video = Vec::new();

    for fr in 0..frames.len() {
        let current_frame = &frames[fr];
        let previous_frame =
            if fr > 0 {
                Some(&frames[fr-1])
            } else {
                None
            };
        let next_frame =
            if fr < frames.len() - 1 && false { // maybe for the future
                Some(&frames[fr+1])
            } else {
                None
            };

        let mut changes = HashMap::new();

        for i in 0..current_frame.len() {
            let (x, y) = index_to_position(i, width);
            if previous_frame.is_some() && next_frame.is_some() {
                if current_frame[i] == previous_frame.unwrap()[i] && current_frame[i] == next_frame.unwrap()[i] {
                    continue;
                }
            } else if previous_frame.is_some() {
                if current_frame[i] == previous_frame.unwrap()[i] {
                    continue;
                }
            } else if next_frame.is_some() {
                if current_frame[i] == next_frame.unwrap()[i] {
                    continue;
                }
            }
            changes.insert((x,y), current_frame[i]);
        }

        video.push(changes);

        log("Frames processed", fr + 1, frame_count);
    }

    return video;
}

fn index_to_position(index: usize, width: usize) -> (usize, usize) {
    let y = index / width;
    let x = index % width;
    (x, y)
}

fn flatten_int(number: u8, bits: u8) -> u8 {
    let bits = bits^2;
    ((number as f64 / bits as f64).round() * bits as f64) as u8
}

fn flatten_color(color: &[u8; 4], bits: u8) -> [u8; 4] {
    let mut output = [color[3]; 4];
    output[0] = flatten_int(color[0], bits);
    output[1] = flatten_int(color[1], bits);
    output[2] = flatten_int(color[2], bits);
    return output;
}

fn log(message: &str, current: usize, total: usize) {
    println!(
        "{}: {}/{} ({}%)",
        message,
        current,
        total,
        100 * current / total,
    );
}
