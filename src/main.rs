use clap::{App, Arg};
use futures::StreamExt;
use futures_timer::Delay;
use rand::random;
use serde_json::json;
use std::i64;
use std::io::Write;

use serde_derive::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::udp::SendHalf;
use tokio::net::UdpSocket;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;
use url::Url;

#[derive(Debug, Deserialize)]
struct Conf {
  filemode: bool,
  rssi_threshold: i32,

  video_sources: Vec<String>,
  lap_sources: Vec<String>,
  laptime_sources: Vec<String>,
}

async fn rssi_timer(udpchanneltx: futures::channel::mpsc::UnboundedSender<String>) {
  loop {
    Delay::new(Duration::from_secs(1)).await;
    udpchanneltx.unbounded_send("S0r\n".to_string()).unwrap();
  }
}

async fn programm_to_udp(
  mut udpchannelrx: futures::channel::mpsc::UnboundedReceiver<String>,
  mut udptx: SendHalf,
) {
  loop {
    let send_data = udpchannelrx.next().await.unwrap();
    let msg = send_data.into_bytes();
    udptx.send(&msg).await.unwrap();
  }
}

async fn udp_comm(appconf: &Conf, senddata: futures::channel::mpsc::UnboundedSender<Message>) {
  let mut drone_active = vec![false, false, false, false, false, false]; // There ia a maximum os 6 receivers

  // Setup the UDP Socket
  let mut udpsocket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
  udpsocket
    .connect("192.168.0.141:9000")
    .await
    .expect("could not connect to udp ");

  let msg = String::from("ok\n").into_bytes();
  udpsocket.send(&msg).await.unwrap();

  let (mut udprx, udptx) = udpsocket.split();

  let (udpchanneltx, udpchannelrx) = futures::channel::mpsc::unbounded();
  tokio::spawn(programm_to_udp(udpchannelrx, udptx));
  tokio::spawn(rssi_timer(udpchanneltx.clone()));

  loop {
    let mut buf: [u8; 500] = [0; 500];
    let len = udprx.recv(&mut buf).await.unwrap();
    let result = Vec::from(&buf[0..len]);

    let display_result = result.clone();
    let result_str = String::from_utf8(display_result).unwrap();

    if result_str.contains("S0R1") {
      /*
      TODO: We should start the racecounter here
      let source_id = "LAPTIME";
      let request = json!({"request-type":"SetTextFreetype2Properties", "source":appconf.la,"message-id": random::<f64>().to_string(), "text": now.elapsed().as_millis().to_string() });
      now = Instant::now();
            senddata
        .unbounded_send(Message::Text(request.to_string()))
        .unwrap();
      */
      if appconf.filemode {
        write_file("Race active".to_string(), "racestate.txt");
        write_file("0".to_string(), "rx1.txt");
        write_file("0".to_string(), "rx2.txt");
        write_file("0".to_string(), "rx3.txt");
      }
    } else if result_str.contains("S0R0") {
      if appconf.filemode {
        write_file("Race inactive".to_string(), "racestate.txt");
      }
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
            let request = json!({"request-type":"SetSourceFilterVisibility", "sourceName":appconf.video_sources[index],"message-id": random::<f64>().to_string(), "filterName":"mask" , "filterEnabled": true  });
            senddata
              .unbounded_send(Message::Text(request.to_string()))
              .unwrap();
            drone_active[index] = false;
          }
        } else {
          // Drone is connected!
          if !drone_active[index] {
            // Send filter off
            let request = json!({"request-type":"SetSourceFilterVisibility", "sourceName":appconf.video_sources[index],"message-id": random::<f64>().to_string(), "filterName":"mask" , "filterEnabled": false  });
            senddata
              .unbounded_send(Message::Text(request.to_string()))
              .unwrap();
            drone_active[index] = true;
          }
        }
        index = index + 1;
      }
    } else if result_str.contains("S0L") {
      // zb    sS1L0000000DAF
      if let Ok(lap_time) = i64::from_str_radix(&result_str[5..13], 16) {
        let lap_seconds = (lap_time as f64) / (1000 as f64);
        if appconf.filemode {
          write_file(lap_seconds.to_string(), "rx1_laptime.txt");
        }
        // TODO: LapTime  to OBS!
      }
      if let Ok(intval) = &result_str[3..5].parse::<i32>() {
        if appconf.filemode {
          write_file((intval + 1).to_string(), "rx1.txt");
        }
        let request = json!({"request-type":"SetTextFreetype2Properties", "source":appconf.lap_sources[0],"message-id": random::<f64>().to_string(), "text": (intval + 1).to_string() });
        senddata
          .unbounded_send(Message::Text(request.to_string()))
          .unwrap();
      }
    } else if result_str.contains("S1L") {
      if let Ok(lap_time) = i64::from_str_radix(&result_str[5..13], 16) {
        let lap_seconds = (lap_time as f64) / (1000 as f64);
        if appconf.filemode {
          write_file(lap_seconds.to_string(), "rx2_laptime.txt");
        }
      }

      if let Ok(intval) = &result_str[3..5].parse::<i32>() {
        if appconf.filemode {
          write_file((intval + 1).to_string(), "rx2.txt");
        }
        let request = json!({"request-type":"SetTextFreetype2Properties", "source":appconf.lap_sources[1],"message-id": random::<f64>().to_string(), "text": (intval + 1).to_string() });
        senddata
          .unbounded_send(Message::Text(request.to_string()))
          .unwrap();
      }
    } else if result_str.contains("S2L") {
      if let Ok(lap_time) = i64::from_str_radix(&result_str[5..13], 16) {
        let lap_seconds = (lap_time as f64) / (1000 as f64);
        if appconf.filemode {
          write_file(lap_seconds.to_string(), "rx3_laptime.txt");
        }
      }

      if let Ok(intval) = &result_str[3..5].parse::<i32>() {
        if appconf.filemode {
          write_file((intval + 1).to_string(), "rx3.txt");
        }
        let request = json!({"request-type":"SetTextFreetype2Properties", "source":appconf.lap_sources[2],"message-id": random::<f64>().to_string(), "text": (intval + 1).to_string() });
        senddata
          .unbounded_send(Message::Text(request.to_string()))
          .unwrap();
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
    .arg(
      Arg::with_name("filemode")
        .short("F")
        .multiple(false)
        .help("Enables File Mode"),
    )
    .get_matches();

  // Gets a value for config if supplied by user, or defaults to "default.conf"
  let config = matches.value_of("config").unwrap_or("default.conf");
  println!("Value for config: {}", config);

  let mut file = File::open("Config.toml").expect("Unable to open the file");
  let mut contents = String::new();
  file
    .read_to_string(&mut contents)
    .expect("Unable to read the file");
  let appconf: Conf = toml::from_str(contents.as_str()).unwrap();
  /*
    let appconf = Conf {
      filemode: matches.is_present("filemode"),
      rssi_threshold: 100,
    };
  */
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

  if appconf.filemode {
    write_file("Race inactive".to_string(), "racestate.txt");
    write_file("0".to_string(), "rx1.txt");
    write_file("0".to_string(), "rx2.txt");
    write_file("0".to_string(), "rx3.txt");

    write_file("-.-".to_string(), "rx1_laptime.txt");
    write_file("-.-".to_string(), "rx2_laptime.txt");
    write_file("-.-".to_string(), "rx3_laptime.txt");
  }

  // Setup websocket for OBS
  let (ws_stream, _) = connect_async(Url::parse("ws://localhost:4444/").unwrap())
    .await
    .expect("Could not connect to OBS");

  println!("Connected to OBS");

  let (ws_write, ws_read) = ws_stream.split();
  let (obstx, obsrx) = futures::channel::mpsc::unbounded();

  let ws_to_stdout = {
    ws_read.for_each(|message| {
      async {
        let data = message.unwrap().into_data();
        println!("Messg");
        tokio::io::stdout().write_all(&data).await.unwrap();
      }
    })
  };
  tokio::spawn(ws_to_stdout);

  let programm_to_ws = obsrx.map(Ok).forward(ws_write);
  tokio::spawn(programm_to_ws);

  println!("Programm initialized!");
  udp_comm(&appconf, obstx).await;
}
