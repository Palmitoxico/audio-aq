/****************************************************************************
 * main.rs
 *
 *   Copyright (C) 2019 Augusto Fraga Giachero. All rights reserved.
 *   Author: Augusto Fraga Giachero <afg@augustofg.net>
 *
 * This file is part of the telab-voip program.
 *
 * telab-voip is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * RFFE is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with telab-voip.  If not, see <https://www.gnu.org/licenses/>.
 *
 ****************************************************************************/

extern crate serialport;
extern crate cpal;
extern crate docopt;
#[macro_use]
extern crate serde_derive;

use docopt::Docopt;
use std::io::{Read, Write, prelude::*, BufReader};
use std::env;
use serialport::prelude::*;
use cpal::traits::{EventLoopTrait, HostTrait};
use std::time::Duration;
use std::process;
use std::net::{TcpListener, TcpStream};

const USAGE: &'static str = "
Telecommunication's Laboratory VoIP Demonstration

Usage:
  __PROGNAME__ start-server -s SERIALPORT [-r SAMPLERATE] [-b BITS] [-p PORT]
  __PROGNAME__ start-client -d DESTINATION [-p PORT]
  __PROGNAME__ (-h | --help)

Options:
  -h --help         Show this screen.
  -s SERIALPORT     Serial port name.
  -d DESTINATION    Server ip address.
  -r SAMPLERATE     Audio sample rate [default: 48000].
  -b BITS           Audio sample resolution in bits [default: 12].
  -p PORT           TCP port [default: 6007].
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_s: String,
    flag_d: String,
    flag_r: u32,
    flag_b: u8,
    flag_p: u16,
    cmd_start_client: bool,
    cmd_start_server: bool,
}

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

fn decode_buffer(inbuf: &[u8], outq: &mut Vec<u8>) {
	for ind in (0..inbuf.len()).step_by(2) {
		if ind + 1 < inbuf.len() {
			let decoded = decode_sample(inbuf[ind], inbuf[ind + 1]);
			outq.extend(&decoded.to_le_bytes());
		}
	}
}

fn server(serialport_name: &String, sample_rate: u32, tcp_port: u16, bits: u8) {
	let serial_settings: SerialPortSettings = Default::default();
    let mut port = match serialport::open_with_settings(&serialport_name, &serial_settings) {
		Ok(p) => p,
		Err(e) => {
			eprintln!("Failed to open serial port '{}': {}", serialport_name, e);
			process::exit(1);
		},
	};

	/*
	 * Set serial port timeout to 2 seconds
	 */
	port.set_timeout(Duration::new(2, 0)).unwrap();

	/*
	 * Inform the sample rate to hardware
	 */
	port.write(sample_rate.to_string().as_bytes()).unwrap();
	port.write(b"\n").unwrap();

	/*
	 * Encapsulates the serial port I/O operations into a BufReader
	 * object to allow efficient data retrival
	 */
	let reader = &mut BufReader::new(port);

	let listener = match TcpListener::bind(format!("0.0.0.0:{}", tcp_port)) {
		Ok(l) => l,
		Err(e) => {
			eprintln!("Failed to bind a socket: {}", e);
			process::exit(1);
		},
	};

	println!("Server listening on port {}", tcp_port);

	for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
				stream.write(&sample_rate.to_le_bytes()).unwrap();
				stream.write(&bits.to_le_bytes()).unwrap();
				let mut buf = Vec::new();
				for line in reader.lines() {
                    match line {
                        Ok(line) => {
                            decode_buffer(line.as_bytes(), &mut buf);

                            /*
                             * Only data via network if the buffer has
                             * more than 512 bytes to avoid excessive
                             * number of small TCP packets
                             * (inefficient)
                             */
                            if buf.len() >= 512 {
                                match stream.write(buf.as_slice()) {
                                    Ok(_) => (),
                                    Err(e) => {
                                        eprintln!("Connection error: {}", e);
                                        break;
                                    },
                                };
                                buf.clear();
                            }
                        },
                        Err(e) => {
                            eprintln!("Serial port error: {}", e);
                            process::exit(1);
                        },
                    }
                }
            }
            Err(e) => {
                println!("Connection error: {}", e);
            }
        }
    }

    /*
     * Close the socket
     */
    drop(listener);
}

fn client(ip_addr: &String, port: u16) {
    let mut stream = match TcpStream::connect(format!("{}:{}", ip_addr, port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error when trying to connect with server: {}", e);
            process::exit(1);
        },
    };

    let mut sample_rate_raw = [0 as u8; 4];
    match stream.read_exact(&mut sample_rate_raw) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Read error: {}", e);
            process::exit(1);
        },
    };

    let mut bits_raw = [0 as u8; 1];
    match stream.read_exact(&mut bits_raw) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Read error: {}", e);
            process::exit(1);
        },
    };

    let _bits = bits_raw[0];

    let sample_rate = u32::from_le_bytes(sample_rate_raw);

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
    let stream_id = match event_loop.build_output_stream(&device, &format) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Failed to open the default audio output device: {}", e);
            process::exit(1);
        },
    };

    /*
     * Encapsulates the tcp socket I/O operations into a BufReader
     * object to allow efficient data retrival
     */
    let reader = &mut BufReader::new(stream);

    event_loop.play_stream(stream_id.clone()).unwrap();

    event_loop.run(move |id, result| {
        let data = match result {
            Ok(data) => data,
            Err(err) => {
                eprintln!("An error occurred on stream {:?}: {}", id, err);
                process::exit(1);
            }
        };

        match data {
            cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer) } => {
                for sample in buffer.chunks_mut(format.channels as usize) {
                    for out in sample.iter_mut() {
                        {
                            let mut buf = [0u8; 2];
                            match reader.read_exact(&mut buf) {
                                Ok(_) => (),
                                Err(e) => {
                                    eprintln!("Read error: {}", e);
                                    process::exit(1);
                                },
                            };
                            let sample = i16::from_le_bytes(buf);
                            *out = sample;
                        }
                    }
                }
            },
            _ => (),
        }
    });
}

fn main() {
    let usage = USAGE.replace("__PROGNAME__", &env::args().nth(0).unwrap());
    let args: Args = Docopt::new(usage).and_then(|d| d.deserialize()).unwrap_or_else(|e| e.exit());

    /*
     * Start a client session or a server session
     */
    if args.cmd_start_server {
        server(&args.flag_s, args.flag_r, args.flag_p, args.flag_b);
    } else if args.cmd_start_client {
        client(&args.flag_d, args.flag_p);
    } else {
        eprintln!("Invalid arguments!");
        process::exit(1);
    }
}
