use std::io::Write;
use std::net;

fn listen(socket: &net::UdpSocket) {
  let mut buf: [u8; 20] = [0; 20];
  let number_of_bytes: usize = 0;
  let mut result: Vec<u8> = Vec::new();
  match socket.recv_from(&mut buf) {
    Ok((number_of_bytes, _)) => {
      result = Vec::from(&buf[0..number_of_bytes]);
    }
    Err(fail) => println!("failed listening {:?}", fail),
  }

  let display_result = result.clone();
  let result_str = String::from_utf8(display_result).unwrap();
  println!("received message: {:?}", result_str);

  if result_str.contains("S0R1") {
    write_file("Race active", "racestate.txt");
    write_file("00", "rx1.txt");
    write_file("00", "rx2.txt");
    write_file("00", "rx3.txt");
  }
  if result_str.contains("S0R0") {
    write_file("Race inactive", "racestate.txt");
  }

  if result_str.contains("S0L") {
    // zb    sS1L0000000DAF
    write_file(&result_str[3..5], "rx1.txt");
  }
  if result_str.contains("S1L") {
    write_file(&result_str[3..5], "rx2.txt");
  }
  if result_str.contains("S2L") {
    write_file(&result_str[3..5], "rx3.txt");
  }
}

fn write_file(text: &str, filename: &str) {
  let mut file = std::fs::File::create(filename).expect("create failed");
  file.write_all(text.as_bytes()).expect("write failed");
  //println!("data written to file");
}

fn main() {
  write_file("Race inactive", "racestate.txt");
  write_file("00", "rx1.txt");
  write_file("00", "rx2.txt");
  write_file("00", "rx3.txt");

  let socket = net::UdpSocket::bind("0.0.0.0:0").expect("failed to bind host socket"); // local bind port

  let msg = String::from("ok").into_bytes();

  socket
    .send_to(&msg, "192.168.0.141:9000")
    .expect("cannot send");

  loop {
    listen(&socket); // this call is blockig
  }
}
