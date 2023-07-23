#![allow(unused_imports)]
use std::{
    io::{BufRead, Write},
    sync::mpsc::channel,
    thread,
};

use anyhow::Ok;
use broadcase_handler::{Broadcast, BroadcastNode};
use echo_handler::{Echo, EchoNode};
use message::{Handler, Init, Message, Payload};
use unique_id_handler::{Generate, UniqueIdNode};

mod broadcase_handler;
mod echo_handler;
mod message;
mod unique_id_handler;
fn main() -> message::Result<()> {
    let mut stdin = std::io::stdin().lock();
    let mut init_message = String::new();
    stdin.read_line(&mut init_message)?;
    let mut stdout = std::io::stdout().lock();

    let init_message = serde_json::from_str::<Message<Init>>(&init_message)?;
    let _ = init_message
        .body
        .data
        .handle(&mut stdout, init_message.clone())?;

    let (tx, rx) = channel();

    drop(stdin);
    let tx_cloned = tx.clone();
    let join_handler = thread::spawn(move || {
        let stdin = std::io::stdin().lock();
        for line in stdin.lines() {
            let incoming = line?.parse::<Message<Broadcast>>()?;
            tx_cloned.send(incoming)?;

            // let _ = broadcast_node.handle(&mut stdout, incoming)?;
        }
        tx_cloned.send(Message {
            src: "Self".to_string(),
            dst: "Self".to_string(),
            body: Payload::new(Broadcast::Quit, None),
        })?;
        Ok(())
    });

    let broadcast_node = BroadcastNode::new(
        match init_message.body.data {
            Init::Init { node_id, .. } => node_id.clone(),
            _ => panic!("First message should be of type echo"),
        },
        tx,
    );

    for message in rx {
        broadcast_node.handle(&mut stdout, message)?;
    }

    let _ = join_handler.join().expect("Main panicked");

    Ok(())
}
