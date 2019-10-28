extern crate serialport;
extern crate cpal;
extern crate queues;

use queues::*;
use std::io::{Write, prelude::*, BufReader};
use std::env;
use serialport::prelude::*;
use cpal::traits::{EventLoopTrait, HostTrait};
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::process;

fn decode_char(c: u8) -> u8 {
	let mut ret: u8 = 0;
	let alpha_len = 'z' as u8 - 'a' as u8;

	if c >= 'A' as u8 && c <= 'Z' as u8 {
		ret = c - 'A' as u8;
	} else if c >= 'a' as u8 && c <= 'z' as u8 {
		ret = c - 'a' as u8 + alpha_len;
	} else if c >= '0' as u8 && c <= '9' as u8 {
		ret = c - '0' as u8 + 2*alpha_len;
	} else if c == '+' as u8 {
		ret = 62;
	} else if c == '/' as u8 {
		ret = 63;
	}
	ret
}

fn decode_sample(l: u8, h: u8) -> i16 {
	let ret = (decode_char(h) as i16) << 6 |
	decode_char(l) as i16;
	(ret - 2048) * 16
}

fn decode_buffer(inbuf: &[u8], outq: &mut Queue<i16>) {
	for ind in (0..inbuf.len()).step_by(2) {
		if ind + 1 < inbuf.len() {
			let decoded = decode_sample(inbuf[ind], inbuf[ind + 1]);
			outq.add(decoded).unwrap();
		}
	}
}

fn main() {
    let argv: Vec<_> = env::args().collect();
	let queue: Queue<i16> = Queue::new();
	let audio_buffer = Arc::new(Mutex::new(queue));

    if argv.len() != 3 {
		eprintln!("Invalid number of arguments");
		process::exit(1);
    }

    let serialport_name = &argv[1];
    let sample_rate_str = &argv[2];
    let sample_rate = (&argv[2]).parse::<u32>().unwrap();

	let host = cpal::default_host();
    let device = match host.default_output_device() {
		Some(dev) => dev,
		None => {
			eprintln!("Failed to find a default audio output device");
			process::exit(1);
		},
	};

    let format = cpal::Format {
		channels: 1,
		sample_rate: cpal::SampleRate(sample_rate),
		data_type: cpal::SampleFormat::I16,
	};
	let event_loop = host.event_loop();
	let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
	event_loop.play_stream(stream_id.clone()).unwrap();

    let serial_settings: SerialPortSettings = Default::default();
    let mut port = match serialport::open_with_settings(serialport_name, &serial_settings) {
		Ok(p) => p,
		Err(e) => {
			eprintln!("Failed to open serial port '{}': {}", serialport_name, e);
			process::exit(1);
		},
	};

	port.set_timeout(Duration::new(2, 0)).unwrap();

	port.write(sample_rate_str.as_bytes()).unwrap();
	port.write(b"\n").unwrap();

	let (tx, rx): (Sender<i32>, Receiver<i32>) = mpsc::channel();
	let audio_buffer_tx = Arc::clone(&audio_buffer);
	thread::spawn(move || {
		let reader = BufReader::new(port);
		for line in reader.lines() {
			let mut buf = audio_buffer_tx.lock().unwrap();
			match line {
				Ok(line) => decode_buffer(line.as_bytes(), &mut buf),
				Err(e) => {
					eprintln!("Serial port error: {}", e);
					tx.send(0).unwrap();
					break;
				},
			}
		}
	});

	event_loop.run(move |id, result| {
        let data = match result {
            Ok(data) => data,
            Err(err) => {
                eprintln!("An error occurred on stream {:?}: {}", id, err);
				process::exit(1);
            }
        };

		match rx.try_recv() {
			Ok(_) => {
				process::exit(1);
			},
			Err(_) => (),
		};

        match data {
            cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer) } => {
                for sample in buffer.chunks_mut(format.channels as usize) {
                    for out in sample.iter_mut() {
						{
							let mut buf = audio_buffer.lock().unwrap();
							match buf.remove() {
								Ok(val) => *out = val,
								Err(_) => break,
							}
						}
                    }
                }
            },
            _ => (),
        }
    });
}
