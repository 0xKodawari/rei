// File just to hold doing it in memory vs writing to disk to see if all this hassle is worth it 

// let start_and_end: Vec<(u128, u128)> = subs
//         .into_par_iter()
//         .map(|sub| {
//             let start_time = sub.start_time.into_duration().as_micros();
//             let end_time = sub.end_time.into_duration().as_micros();
//             let (a, b) = if start_time > end_time {
//                 (end_time, start_time)
//             } else {
//                 (start_time, end_time)
//             };
//             (a, b)
//         })
//         .collect();
//     let mut index = 0;
//     let mut outputs = HashMap::new();
//     let buffer = Arc::new(Mutex::new(Vec::new()));

//     let contexts: Vec<_> = start_and_end
//         .into_iter()
//         .map(|times| {
//             let write_callback: Box<dyn FnMut(&[u8]) -> i32> = {
//                 let buffer = Arc::clone(&buffer);
//                 let mut written = false;
//                 Box::new(move |buf: &[u8]| {
//                     let mut buffer = buffer.lock().unwrap();
//                     if !written && buf.len() > 44 {
//                         let mut data = buf[44..].to_vec();
//                         if data.len() % 4 != 0 {
//                             let pad = 4 - (data.len() % 4);
//                             data.extend(std::iter::repeat(0u8).take(pad));
//                         }
//                         buffer.extend_from_slice(&data);
//                         written = true;
//                     } else {
//                         buffer.extend_from_slice(buf);
//                     }

//                     buf.len() as i32
//                 })
//             };
//             let mut output_context: Output = write_callback.into();
//             output_context = output_context.set_format("wav");
//             let duration_sec = (times.1 - times.0) as f64 / 1_000_000.0;
//             let fade_dur = duration_sec.min(0.025);
//             let fade_filter = format!(
//                 "afade=t=in:ss=0:d={fade_dur:.3},afade=t=out:st={:.3}:d={fade_dur:.3}",
//                 duration_sec - fade_dur
//             );
//             // need to update this not just automatically use .wav
//             let output = format!("tmp_audio_dir/tmp_output{index}.wav");
//             let context = FfmpegContext::builder()
//                 .input(
//                     Input::from(file)
//                         .set_start_time_us(times.0 as i64)
//                         .set_stop_time_us(times.1 as i64),
//                 )
//                 .filter_desc(fade_filter)
//                 .output(output_context)
//                 .build()
//                 .unwrap();
//             outputs.insert(index, output.clone());
//             index += 1;
//             (context, output.clone(), index)
//         })
//         .collect();