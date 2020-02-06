use clap::{App, Arg};
use futures::StreamExt;
use futures_timer::Delay;
use std::i64;
use std::io::Write;

use serde_derive::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
//use tokio::io::AsyncWriteExt;
use tokio::net::udp::SendHalf;
use tokio::net::UdpSocket;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;
use url::Url;

mod obsws;

#[derive(Debug, Deserialize)]
struct Conf {
  filemode: bool,
  enable_websocket: bool,
  rssi_threshold: i32,
  race_status_source: String,
  race_time_source: Option<String>,

  obs_websocket_url: Option<String>,
  chorus_udp_url: Option<String>,
  obs_mask_filter_name: Option<String>,

  video_sources: Vec<String>,
  lap_sources: Vec<String>,
  laptime_sources: Vec<String>,
}

async fn rssi_timer(udpchanneltx: futures::channel::mpsc::UnboundedSender<String>) {
  loop {
    Delay::new(Duration::from_millis(500)).await;
    udpchanneltx
      .unbounded_send("S0r\n".to_string())
      .unwrap_or_else(|err| {
        eprintln!("Could not request RSSI from Chorus: {}", err);
      });
  }
}

async fn programm_to_udp(
  mut udpchannelrx: futures::channel::mpsc::UnboundedReceiver<String>,
  mut udptx: SendHalf,
) {
  loop {
    let send_data = udpchannelrx.next().await.unwrap();
    let msg = send_data.into_bytes();
    udptx.send(&msg).await.unwrap_or_else(|err| {
      eprintln!("Could not send to Chorus: {}", err);
      return 0;
    });
  }
}

async fn udp_comm(appconf: &Conf, senddata: futures::channel::mpsc::UnboundedSender<Message>) {
  let mut drone_active = vec![false, false, false, false, false, false]; // There ia a maximum os 6 receivers
  let race_timer_state = Arc::new(AtomicBool::new(false));
  let race_timer_state_clone = race_timer_state.clone();
  // Setup the UDP Socket
  let mut udpsocket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
  udpsocket
    .connect(
      &appconf
        .chorus_udp_url
        .as_ref()
        .unwrap_or(&"192.168.0.141:9000".to_string()),
    )
    .await
    .expect("could not connect to udp ");

  let msg = String::from("ok\n").into_bytes();
  udpsocket.send(&msg).await.unwrap_or_else(|err| {
    eprintln!("Could not send to Chorus: {}", err);
    return 0;
  });

  let (mut udprx, udptx) = udpsocket.split();

  let (udpchanneltx, udpchannelrx) = futures::channel::mpsc::unbounded();
  tokio::spawn(programm_to_udp(udpchannelrx, udptx));
  tokio::spawn(rssi_timer(udpchanneltx.clone()));

  if let Some(race_time_source) = &appconf.race_time_source {
    tokio::spawn(obsws::race_timer(
      senddata.clone(),
      race_time_source.clone(),
      race_timer_state_clone,
    ));
  }

  loop {
    let mut buf: [u8; 500] = [0; 500];
    let len = udprx.recv(&mut buf).await.unwrap();
    let result = Vec::from(&buf[0..len]);

    let display_result = result.clone();
    let result_str = String::from_utf8(display_result).unwrap();

    if result_str.contains("S0R1") {
      race_timer_state.store(true, Ordering::Relaxed);
      println!("Set race bool");
      if appconf.filemode {
        write_file("Race active".to_string(), "racestate.txt");
        write_file("0".to_string(), "rx1.txt");
        write_file("0".to_string(), "rx2.txt");
        write_file("0".to_string(), "rx3.txt");
      }
      obsws::set_text(
        &senddata,
        &appconf.race_status_source,
        &"Race active".to_string(),
      );
      for i in 0..2 {
        obsws::set_text(&senddata, &appconf.lap_sources[i], &"0".to_string());
        obsws::set_text(
          &senddata,
          &appconf.laptime_sources[i],
          &"00:00.000".to_string(),
        );
      }
    } else if result_str.contains("S0R0") {
      race_timer_state.store(false, Ordering::Relaxed);
      if appconf.filemode {
        write_file("Race inactive".to_string(), "racestate.txt");
      }
      obsws::set_text(
        &senddata,
        &appconf.race_status_source,
        &"Race inactive".to_string(),
      );
    } else if result_str.contains("S0r") {
      //S0r004A\nS1r0044\nS2r0044
      let rxes = result_str.split("\n");
      let mut index = 0;
      for node in rxes {
        if node.len() < 7 {
          continue;
        };

        let rssi = i32::from_str_radix(&node[3..7], 16).unwrap_or(0);

        if rssi < appconf.rssi_threshold {
          // Drone is disconnected
          if drone_active[index] {
            // Send filter on
            if &appconf.video_sources.len() <= &index {
              obsws::set_mask(
                &senddata,
                &appconf.video_sources[index],
                &appconf.obs_mask_filter_name,
                true,
              );
            } else {
              eprintln!(
                "No sourcename provided for rssi value string: {}, index {}",
                node, index
              )
            }
            drone_active[index] = false;
          }
        } else {
          // Drone is connected!
          if !drone_active[index] {
            // Send filter off
            if &appconf.video_sources.len() > &index {
              obsws::set_mask(
                &senddata,
                &appconf.video_sources[index],
                &appconf.obs_mask_filter_name,
                false,
              );
            } else {
              eprintln!(
                "No sourcename provided for rssi value string: {}, index {}",
                node, index
              )
            }
            drone_active[index] = true;
          }
        }
        index = index + 1;
      }
    } else if result_str.contains("L") {
      // zb    sS1L0000000DAF
      // now get the index
      if let Ok(rx_number) = &result_str[1..2].parse::<i32>() {
        if let Ok(lap_time) = i64::from_str_radix(&result_str[5..13], 16) {
          if &appconf.laptime_sources.len() <= &(*rx_number as usize) {
            eprintln!("No Sourcename provided for RX {} lapduratio", &rx_number);
            continue;
          } else {
            let lap_duration = Duration::from_millis(lap_time as u64);
            let mut lap_seconds = lap_duration.as_secs();
            let lap_minutes = lap_seconds / 60;
            lap_seconds = lap_seconds - lap_minutes * 60;
            let laptime_string = format!(
              "{:0<2}:{:0<2}.{:0<3}",
              lap_minutes,
              lap_seconds,
              lap_duration.subsec_millis()
            );
            obsws::set_text(&senddata, &appconf.laptime_sources[0], &laptime_string);
            if appconf.filemode {
              write_file(laptime_string, &format!("rx{}_laptime.txt", rx_number + 1));
            }
          }
        }
        if let Ok(lapcount) = &result_str[3..5].parse::<i32>() {
          if &appconf.lap_sources.len() <= &(*rx_number as usize) {
            eprintln!("No Sourcename provided for RX {} lapcount", &rx_number);
            continue;
          }
          if appconf.filemode {
            write_file(
              (lapcount + 1).to_string(),
              &format!("rx{}.txt", rx_number + 1),
            );
          }
          obsws::set_text(
            &senddata,
            &appconf.lap_sources[(*rx_number as usize)],
            &(lapcount + 1).to_string(),
          );
        }
      }
    } else {
      println!("Received unknown message from Chorus: {:?}", result_str);
    }
  }
}

fn write_file(text: String, filename: &str) {
  let mut file = std::fs::File::create(filename).expect("create failed");
  file.write_all(text.as_bytes()).expect("write failed");
}

#[tokio::main]
async fn main() {
  let matches = App::new("chorusOBSsync")
    .version("1.0")
    .author("Lukas B. <lukas@lbsfilm.at>")
    .about("Get data from the Chorus32 Laptimer Project and use it to control OBS")
    .arg(
      Arg::with_name("config")
        .short("c")
        .long("config")
        .value_name("FILE")
        .help("Sets a custom config file")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("v")
        .short("v")
        .multiple(true)
        .help("Sets the level of verbosity"),
    )
    .arg(
      Arg::with_name("filemode")
        .short("F")
        .multiple(false)
        .help("Enables File Mode"),
    )
    .get_matches();

  // Gets a value for config if supplied by user, or defaults to "default.conf"
  let configfile = matches.value_of("config").unwrap_or("Config.toml");
  println!("Using config file: {}", configfile);

  let mut file = File::open(&configfile).expect("Unable to open config file");
  let mut contents = String::new();
  file
    .read_to_string(&mut contents)
    .expect("Unable to read the file");
  let appconf: Conf = toml::from_str(contents.as_str()).unwrap();

  // Vary the output based on how many times the user used the "verbose" flag
  // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
  match matches.occurrences_of("v") {
    0 => println!("No verbose info"),
    1 => println!("Verbose UDP"),
    2 => println!("Verbose UDP + Websocket"),
    3 | _ => println!("Don't be crazy"),
  }

  if appconf.filemode {
    write_file("Race inactive".to_string(), "racestate.txt");
    write_file("0".to_string(), "rx1.txt");
    write_file("0".to_string(), "rx2.txt");
    write_file("0".to_string(), "rx3.txt");

    write_file("-.-".to_string(), "rx1_laptime.txt");
    write_file("-.-".to_string(), "rx2_laptime.txt");
    write_file("-.-".to_string(), "rx3_laptime.txt");
  }

  if !appconf.enable_websocket {
    println!("Programm initialized! (Without websocket connection)");
    let (obstx, obsrx) = futures::channel::mpsc::unbounded(); // creating without a place to send the packets
    tokio::spawn(obschan_to_nowhere(obsrx));

    udp_comm(&appconf, obstx).await;
    std::process::exit(0);
  }

  // Setup websocket for OBS
  let (ws_stream, _) = connect_async(
    Url::parse(
      &appconf
        .obs_websocket_url
        .as_ref()
        .unwrap_or(&"ws://localhost:4444/".to_string()),
    )
    .expect("Invalid Websocket URL format"),
  )
  .await
  .expect("Could not connect to OBS");

  println!("Connected to OBS");

  let (ws_write, ws_read) = ws_stream.split();
  let (obstx, obsrx) = futures::channel::mpsc::unbounded();

  let ws_to_stdout = {
    ws_read.for_each(|message| {
      async {
        let data = message.unwrap().into_data();

        let data_string = match std::str::from_utf8(&data) {
          Ok(v) => v,
          Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };

        if let Ok(untyped_json_value) = serde_json::from_str(data_string) {
          let json_value: serde_json::Value = untyped_json_value;
          if json_value["status"].as_str().is_some()
            && json_value["status"].as_str().unwrap() == "error"
          {
            eprintln!("OBBS Error: {}", json_value["error"]);
          }
        // TODO: Check appconfig verbosity
        //tokio::io::stdout().write_all(&data).await.unwrap();
        } else {
          eprintln!("Could not parse json value from obs");
        }
      }
    })
  };
  tokio::spawn(ws_to_stdout);

  let programm_to_ws = obsrx.map(Ok).forward(ws_write);
  tokio::spawn(programm_to_ws);

  println!("Programm initialized!");
  udp_comm(&appconf, obstx).await;
}

async fn obschan_to_nowhere(mut obsrx: futures::channel::mpsc::UnboundedReceiver<Message>) {
  loop {
    obsrx.next().await.unwrap();
  }
}
