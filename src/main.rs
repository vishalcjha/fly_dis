use std::io::{BufRead, Write};

use echo_handler::{Echo, EchoNode};
use message::{Handler, Init, Message};

mod echo_handler;
mod message;
fn main() -> message::Result<()> {
    let mut stdin = std::io::stdin().lock();
    let mut init_message = String::new();
    stdin.read_line(&mut init_message)?;

    let init_message = serde_json::from_str::<Message<Init>>(&init_message)?;
    let init_response = init_message.body.data.handle(&init_message)?;

    let mut stdout = std::io::stdout().lock();
    // println!("Finished writting response {:?}", init_response);
    serde_json::to_writer(&mut stdout, &init_response)?;
    stdout.write(b"\n")?;

    let echo_node = EchoNode::new(match init_message.body.data {
        Init::Init { node_id, .. } => node_id.clone(),
        _ => panic!("First message should be of type echo"),
    });
    for line in stdin.lines() {
        let incoming = line?.parse::<Message<Echo>>()?;
        // println!("=========== Received {:?} ==================", incoming);
        let response = echo_node.handle(&incoming)?;
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write(b"\n")?;
    }
    Ok(())
}
