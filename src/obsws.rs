use futures_timer::Delay;
use rand::random;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tungstenite::protocol::Message;

pub fn set_text(
  wschannel: &futures::channel::mpsc::UnboundedSender<Message>,
  source: &String,
  text: &String,
) {
  let request = json!({"request-type":"SetTextFreetype2Properties", "source":source,"message-id": random::<f64>().to_string(), "text": text });
  wschannel
    .unbounded_send(Message::Text(request.to_string()))
    .unwrap_or_else(|err| {
      eprintln!("Could not send to OBS: {}", err);
    });
}

pub fn set_mask(
  wschannel: &futures::channel::mpsc::UnboundedSender<Message>,
  source: &String,
  mask_name: &Option<String>,
  active: bool,
) {
  let request = json!({"request-type":"SetSourceFilterVisibility", "sourceName":source,"message-id": random::<f64>().to_string(), "filterName":mask_name.as_ref()
  .unwrap_or(&"mask".to_string()) , "filterEnabled": active  });
  wschannel
    .unbounded_send(Message::Text(request.to_string()))
    .unwrap_or_else(|err| {
      eprintln!("Could not send to OBS: {}", err);
    });
}

pub async fn race_timer(
  wschannel: futures::channel::mpsc::UnboundedSender<Message>,
  source: String,
  race_timer_state: Arc<AtomicBool>,
) {
  loop {
    Delay::new(Duration::from_millis(100)).await;
    let now = Instant::now();
    while race_timer_state.load(Ordering::Relaxed) {
      Delay::new(Duration::from_millis(200)).await;
      let request = json!({"request-type":"SetTextFreetype2Properties", "source":source,"message-id": random::<f64>().to_string(), "text": now.elapsed().as_millis().to_string() });
      wschannel
        .unbounded_send(Message::Text(request.to_string()))
        .unwrap_or_else(|err| {
          eprintln!("Could not send to OBS: {}", err);
        });
    }
  }
}
