use rand::random;
use serde_json::json;
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
