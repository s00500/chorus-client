use rand::random;
use serde_json::json;
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tungstenite::{connect, Error, Message, Result};
use url::Url;

fn main() {
  let now = Instant::now();

  let (mut socket, _) =
    connect(Url::parse("ws://localhost:4444/").unwrap()).expect("Could not connect");
  let source_id = "LAPTIME";
  let new_text = "rusty";
  let handler = thread::spawn(|| loop {
    let request = json!({"request-type":"SetTextFreetype2Properties", "source":source_id,"message-id": random::<f64>().to_string(), "text": now.elapsed().as_millis().to_string() });
    socket
      .write_message(Message::Text(request.to_string()))
      .unwrap();
    println!("{}", now.elapsed().as_secs());
    sleep(Duration::from_millis(100));
  });

  /*
  loop {
    let msg = socket.read_message().unwrap();
    let text = msg.into_text().unwrap_or("".to_string());
    println!("{}", text);
  }
  */
  //socket.close(None).unwrap();
}
