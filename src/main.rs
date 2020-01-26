extern crate clap;

use clap::{App, Arg};
use futures::{future, pin_mut, StreamExt};
use rand::random;
use serde_json::json;
use std::i64;
use std::io;
use std::io::Write;
use std::net;
use std::thread::spawn;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;
use url::Url;

fn listen(socket: &net::UdpSocket) {
  let mut buf: [u8; 20] = [0; 20];
  let mut result: Vec<u8> = Vec::new();
  match socket.recv_from(&mut buf) {
    Ok((number_of_bytes, _)) => {
      result = Vec::from(&buf[0..number_of_bytes]);
    }
    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
      return ();
    }
    Err(e) => println!("failed listening {:?}", e),
  }

  let display_result = result.clone();
  let result_str = String::from_utf8(display_result).unwrap();
  println!("received message: {:?}", result_str);

  if result_str.contains("S0R1") {
    write_file("Race active".to_string(), "racestate.txt");
    write_file("0".to_string(), "rx1.txt");
    write_file("0".to_string(), "rx2.txt");
    write_file("0".to_string(), "rx3.txt");
  }
  if result_str.contains("S0R0") {
    write_file("Race inactive".to_string(), "racestate.txt");
  }

  if result_str.contains("S0L") {
    // zb    sS1L0000000DAF
    let lap_time = i64::from_str_radix(&result_str[5..13], 16).unwrap_or(-1);
    if lap_time != -1 {
      let lap_seconds = (lap_time as f64) / (1000 as f64);
      write_file(lap_seconds.to_string(), "rx1_laptime.txt");
    }
    let intval = &result_str[3..5].parse::<i32>().unwrap_or(-1);
    if *intval != -1 {
      write_file((intval + 1).to_string(), "rx1.txt");
    }
  }
  if result_str.contains("S1L") {
    let lap_time = i64::from_str_radix(&result_str[5..13], 16).unwrap_or(-1);
    if lap_time != -1 {
      let lap_seconds = (lap_time as f64) / (1000 as f64);
      write_file(lap_seconds.to_string(), "rx2_laptime.txt");
    }
    let intval = &result_str[3..5].parse::<i32>().unwrap_or(-1);
    if *intval != -1 {
      write_file((intval + 1).to_string(), "rx2.txt");
    }
  }
  if result_str.contains("S2L") {
    let lap_time = i64::from_str_radix(&result_str[5..13], 16).unwrap_or(-1);
    if lap_time != -1 {
      let lap_seconds = (lap_time as f64) / (1000 as f64);
      write_file(lap_seconds.to_string(), "rx3_laptime.txt");
    }
    let intval = &result_str[3..5].parse::<i32>().unwrap_or(-1);
    if *intval != -1 {
      write_file((intval + 1).to_string(), "rx3.txt");
    }
  }
}

fn write_file(text: String, filename: &str) {
  let mut file = std::fs::File::create(filename).expect("create failed");
  file.write_all(text.as_bytes()).expect("write failed");
  //println!("data written to file");
}

#[tokio::main]
async fn main() {
  let matches = App::new("chorusOBSsync")
    .version("1.0")
    .author("Lukas B. <lukas@lbsfilm.at>")
    .about("Get data from the CHorus32 Laptimer Project and use it to control OBS")
    .arg(
      Arg::with_name("config")
        .short("c")
        .long("config")
        .value_name("FILE")
        .help("Sets a custom config file")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("INPUT")
        .help("Sets the input file to use")
        .required(false)
        .index(1),
    )
    .arg(
      Arg::with_name("v")
        .short("v")
        .multiple(true)
        .help("Sets the level of verbosity"),
    )
    .get_matches();

  // Gets a value for config if supplied by user, or defaults to "default.conf"
  let config = matches.value_of("config").unwrap_or("default.conf");
  println!("Value for config: {}", config);

  // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
  // required we could have used an 'if let' to conditionally get the value)
  if let Some(input) = matches.value_of("INPUT") {
    println!("Using input file: {}", input);
  }

  // Vary the output based on how many times the user used the "verbose" flag
  // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
  match matches.occurrences_of("v") {
    0 => println!("No verbose info"),
    1 => println!("Some verbose info"),
    2 => println!("Tons of verbose info"),
    3 | _ => println!("Don't be crazy"),
  }

  //////////----------------
  write_file("Race inactive".to_string(), "racestate.txt");
  write_file("0".to_string(), "rx1.txt");
  write_file("0".to_string(), "rx2.txt");
  write_file("0".to_string(), "rx3.txt");

  write_file("-.-".to_string(), "rx1_laptime.txt");
  write_file("-.-".to_string(), "rx2_laptime.txt");
  write_file("-.-".to_string(), "rx3_laptime.txt");

  // Setup websocket for OBS
  let mut now = Instant::now();

  let source_id = "LAPTIME";

  let (ws_stream, _) = connect_async(Url::parse("ws://localhost:4444/").unwrap())
    .await
    .expect("Could not connect to OBS");
  println!("WebSocket handshake has been successfully completed");

  let (write, read) = ws_stream.split();
  let (obstx, obsrx) = futures::channel::mpsc::unbounded();

  let ws_to_stdout = {
    read.for_each(|message| {
      async {
        let data = message.unwrap().into_data();
        tokio::io::stdout().write_all(&data).await.unwrap();
      }
    })
  };

  let stdin_to_ws = obsrx.map(Ok).forward(write);
  pin_mut!(stdin_to_ws, ws_to_stdout);
  future::select(stdin_to_ws, ws_to_stdout).await;

  // Setup the UDP Socket
  let udpsocket = net::UdpSocket::bind("0.0.0.0:0").expect("failed to bind host udp socket"); // local bind port
  udpsocket.set_nonblocking(true).unwrap();

  let msg = String::from("ok").into_bytes();
  udpsocket
    .send_to(&msg, "192.168.0.141:9000")
    .expect("cannot send");
  loop {
    listen(&udpsocket);

    if now.elapsed().as_secs() >= 5 {
      let request = json!({"request-type":"SetTextFreetype2Properties", "source":source_id,"message-id": random::<f64>().to_string(), "text": now.elapsed().as_millis().to_string() });
      obstx
        .unbounded_send(Message::Text(request.to_string()))
        .unwrap();
      println!("{}", now.elapsed().as_secs());
      now = Instant::now();
    }
  }
}
