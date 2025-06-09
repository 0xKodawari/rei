use anyhow::Result;
use ez_ffmpeg::{FfmpegContext, FfmpegScheduler, Input, Output};
use rand::Rng;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use srtparse::from_file;
use std::collections::HashMap;
use std::fs::{read_dir, remove_file};
use std::path::PathBuf;
use std::{
    fs::{File, remove_dir_all},
    io::Write,
    path::Path,
    sync::Mutex,
};

#[derive(Debug, Clone, Copy)]
pub enum AudioFileType {
    Mp3,
    Wav,
    Opus,
    Ogg,
    Aac,
}
#[derive(Debug, Clone, Copy)]
pub enum SubtitleFileType {
    Srt,
    Ass,
}

impl SubtitleFileType {
    fn to_str(self) -> &'static str {
        match self {
            SubtitleFileType::Srt => "srt",
            SubtitleFileType::Ass => "ass",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VideoFileType {
    Mkv,
}

impl VideoFileType {
    fn to_str(self) -> &'static str {
        match self {
            VideoFileType::Mkv => "mkv",
        }
    }
}

impl AudioFileType {
    fn as_str(self) -> &'static str {
        match self {
            AudioFileType::Mp3 => "mp3",
            AudioFileType::Wav => "wav",
            AudioFileType::Opus => "opus",
            AudioFileType::Ogg => "ogg",
            AudioFileType::Aac => "aac",
        }
    }
}

pub fn generate_audio_files(file: &str, subtitles: String, output_name: &str) -> Result<()> {
    let start = std::time::Instant::now();
    // kills process if file exists already so handle this
    let mut num = rand::rng();
    let num: u32 = num.random();
    let dir_name = format!("tmp_audio_dir{num}",);
    std::fs::create_dir(dir_name.clone())?;
    let subs = from_file(subtitles).expect("failed to parse subtitles");
    let start_and_end: Vec<(u128, u128)> = subs
        .into_par_iter()
        .map(|sub| {
            let start_time = sub.start_time.into_duration().as_micros();
            let end_time = sub.end_time.into_duration().as_micros();
            let (a, b) = if start_time > end_time {
                (end_time, start_time)
            } else {
                (start_time, end_time)
            };
            (a, b)
        })
        .collect();
    let mut index = 0;
    let mut outputs = HashMap::new();

    let contexts: Vec<_> = start_and_end
        .into_iter()
        .map(|times| {
            // need to update this not just automatically use .wav
            let output = format!("{dir_name}/tmp_output{index}.wav");
            let context = FfmpegContext::builder()
                .input(
                    Input::from(file)
                        .set_start_time_us(times.0 as i64)
                        .set_stop_time_us(times.1 as i64),
                )
                .filter_desc("anull")
                .output(Output::from(output.clone()))
                .build()
                .unwrap();
            outputs.insert(index, output.clone());
            index += 1;
            (context, output.clone(), index)
        })
        .collect();
    let failed_outputs = Mutex::new(vec![]);
    dbg!("running scheduler");
    contexts.into_iter().for_each(|ctx| {
        let scheduler = FfmpegScheduler::new(ctx.0).start();
        match scheduler {
            Ok(scheduler) => {
                let waiting_result = scheduler.wait();
                match waiting_result {
                    Ok(_) => (),
                    Err(err) => {
                        failed_outputs.lock().unwrap().push((ctx.1, ctx.2));
                        println!("Error: {err:#?}");
                    }
                }
            }
            Err(err) => {
                println!("Error: {err:#?}");
            }
        }
    });

    let inner = failed_outputs.into_inner().unwrap();
    inner.iter().for_each(|values| {
        let file_path = outputs.remove(&values.1);
        match file_path {
            Some(path) => {
                let _ = std::fs::remove_file(path);
            }
            None => (),
        }
    });
    let successful_files: Vec<String> = outputs.iter().map(|val| val.1.to_owned()).collect();
    let files = sort_files(successful_files);
    dbg!("condensing files");
    let result = condense_files(files, output_name, num);
    match result {
        Ok(_) => (),
        Err(err) => {
            println!("Error condensing file: {}", err);
        }
    }
    let duration = start.elapsed();
    println!("Duration {:#?}", duration);
    clean_up(&dir_name)
}

pub fn generate_multi_audio_files(
    files: Vec<&str>,
    subtitles: Vec<String>,
    output_names: Vec<&str>,
) -> Result<()> {
    files
        .into_par_iter()
        .zip(subtitles)
        .zip(output_names)
        .for_each(|((a, b), c)| {
            let res = generate_audio_files(a, b, c);
            match res {
                Ok(_) => (),
                // We do not want to shut down the whole process but just see which ones failed
                Err(err) => {
                    println!("File: {} failed with Error: {}", a, err);
                }
            }
        });
    Ok(())
}

pub fn condense_files(files: Vec<String>, output_file: &str, num: u32) -> Result<()> {
    let mut filelist = File::create(format!("filelist{num}.txt"))?;
    for path in files {
        writeln!(filelist, "file '{}'", path)?;
    }

    // 2. Run ffmpeg with -f concat
    let _status = std::process::Command::new("ffmpeg")
        .args([
            "-loglevel",
            "quiet",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            format!("filelist{num}.txt").as_str(),
            "-c",
            "copy",
            output_file,
        ])
        .status()?;
    remove_file(format!("filelist{num}.txt"))?;
    Ok(())
}

pub fn change_audio_format(
    input: &str,
    output: &str,
    old_format: AudioFileType,
    new_format: AudioFileType,
) -> Result<()> {
    let input_path = format!("{}.{}", input, old_format.as_str());
    let output_path = format!("{}.{}", output, new_format.as_str());
    let context = FfmpegContext::builder()
        .input(input_path)
        .output(output_path)
        .build()?;
    let _ = FfmpegScheduler::new(context).start()?.wait();
    Ok(())
}

pub struct Files {
    pub subtitle_file_name: Vec<String>,
    pub video_file_names: Vec<String>,
}

pub fn sort_directory(
    path: &str,
    subtitle_format: SubtitleFileType,
    video_format: VideoFileType,
) -> Result<Files> {
    let mut subtitles = vec![];
    let mut videos = vec![];
    match read_dir(path) {
        Ok(path) => {
            path.for_each(|entry| {
                let path = match entry.ok() {
                    Some(val) => val.path(),
                    None => {
                        panic!("invalid directory path");
                    }
                };
                //let path = entry.ok().unwrap().path();
                match path.extension() {
                    Some(ext) => {
                        let ext = ext.to_str();
                        match ext {
                            Some(inner) => match inner {
                                inner if inner == subtitle_format.to_str() => {
                                    subtitles.push(path);
                                }
                                inner if inner == video_format.to_str() => {
                                    videos.push(path);
                                }
                                _ => unreachable!(),
                            },
                            None => (),
                        }
                    }
                    None => (),
                }
            })
        }
        Err(_) => (),
    }

    let subtitles = sort_paths(&mut subtitles);
    let videos = sort_paths(&mut videos);

    Ok(Files {
        subtitle_file_name: subtitles,
        video_file_names: videos,
    })
}

fn sort_paths(paths: &mut Vec<PathBuf>) -> Vec<String> {
    paths.sort_by_key(|p| extract_index(p));
    let files = paths
        .into_iter()
        .map(|file| file.to_str().unwrap().to_string())
        .collect();
    files
}

fn sort_files(files_names: Vec<String>) -> Vec<String> {
    let mut tmp_fills: Vec<_> = files_names
        .iter()
        .map(|p| PathBuf::from(p.as_str()))
        .collect();
    let mut files = vec![];
    tmp_fills.sort_by_key(|p| extract_index(p));
    tmp_fills
        .into_iter()
        .for_each(|p| files.push(p.to_str().unwrap().to_string()));
    files
}

pub fn generate_indexed_output_files(
    output_name: &str,
    file_type: AudioFileType,
    len: usize,
) -> Vec<String> {
    let mut names = vec![];
    for i in 0..len {
        names.push(format!("{}{}.{}", output_name, i, file_type.as_str()));
    }
    names
}
fn extract_index<P: AsRef<Path>>(path: P) -> usize {
    let filename = path.as_ref().file_stem().unwrap().to_string_lossy();
    let digits: String = filename
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    digits.parse::<usize>().unwrap_or(0)
}

// Helper function to clean all of the tempory files after creating all the condensed audio
// The temporary files are places into a directory so we simply remove that
fn clean_up(path: &str) -> Result<()> {
    remove_dir_all(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {}
