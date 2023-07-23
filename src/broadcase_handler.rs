#![allow(dead_code)]
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    io,
    sync::{
        atomic::AtomicBool,
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_with::DurationMilliSeconds;

use crate::message::{self, Handler, Message, Payload, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Broadcast {
    Broadcast {
        message: usize,
    },
    BroadcastOk {
        in_reply_to: usize,
    },
    Read,
    ReadOk {
        messages: HashSet<usize>,
        in_reply_to: usize,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk {
        in_reply_to: usize,
    },
    TriggerGossip,
    Gossip {
        seen: HashSet<usize>,
    },
    Quit,
}

#[derive(Debug)]
pub struct BroadcastNode {
    node_id: String,
    received_messages: RefCell<HashSet<usize>>,
    topology: RefCell<Vec<String>>,
    gossip_handler: JoinHandle<()>,
    quit: Arc<AtomicBool>,
}

impl BroadcastNode {
    pub fn new(node_id: String, tx: Sender<Message<Broadcast>>) -> Self {
        let quit_status = Arc::new(AtomicBool::new(false));

        let gossip_handler = BroadcastNode::start_gossip_signal_producer(tx, quit_status.clone());

        BroadcastNode {
            node_id,
            received_messages: RefCell::default(),
            topology: RefCell::default(),
            gossip_handler,
            quit: quit_status,
        }
    }

    fn start_gossip_signal_producer(
        tx: Sender<Message<Broadcast>>,
        quit_status: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        thread::spawn(move || loop {
            if quit_status.load(std::sync::atomic::Ordering::Acquire) {
                break;
            }
            if let Err(_) = tx.send(Message {
                src: "Self".to_string(),
                dst: "Self".to_string(),
                body: Payload {
                    data: Broadcast::TriggerGossip {},
                    msg_id: None,
                },
            }) {
                break;
            }
            // we are parking not sleeping as it will allow node to close ASAP.
            thread::park_timeout(Duration::from_millis(500));
        })
    }
}

impl Handler<Broadcast> for BroadcastNode {
    fn handle(
        &self,
        writer: &mut dyn io::Write,
        message: message::Message<Broadcast>,
    ) -> message::Result<()> {
        let broadcast_reponse = match message.body.data {
            Broadcast::Broadcast { message: incoming } => {
                // No action when message is seen.
                if !self.received_messages.borrow().contains(&incoming) {
                    self.received_messages.borrow_mut().insert(incoming);
                }

                Some(Broadcast::BroadcastOk {
                    in_reply_to: message.body.msg_id.unwrap_or(1),
                })
            }
            Broadcast::BroadcastOk { in_reply_to } => Some(Broadcast::BroadcastOk { in_reply_to }),
            Broadcast::Read => Some(Broadcast::ReadOk {
                messages: self.received_messages.borrow().clone(),
                in_reply_to: message.body.msg_id.unwrap_or(1),
            }),
            Broadcast::ReadOk {
                messages,
                in_reply_to,
            } => Some(Broadcast::ReadOk {
                messages,
                in_reply_to,
            }),
            Broadcast::Topology { mut topology } => {
                // update topology of current node with its neighbor.
                if let Some(neighbours) = topology.remove(&self.node_id) {
                    self.topology.borrow_mut().clear();
                    self.topology.borrow_mut().extend(neighbours.into_iter());
                }
                Some(Broadcast::TopologyOk {
                    in_reply_to: message.body.msg_id.unwrap_or(1),
                })
            }
            Broadcast::TopologyOk { in_reply_to } => Some(Broadcast::TopologyOk { in_reply_to }),
            Broadcast::Gossip { seen } => {
                self.received_messages.borrow_mut().extend(seen.into_iter());
                None
            }
            Broadcast::TriggerGossip => {
                for neighbor in self.topology.borrow().iter() {
                    serde_json::to_writer(
                        &mut *writer,
                        &Message::new(
                            self.node_id.clone(),
                            neighbor.clone(),
                            Payload {
                                data: Broadcast::Gossip {
                                    seen: self.received_messages.borrow().clone(),
                                },
                                msg_id: None,
                            },
                        ),
                    )?;
                    writer.write_all(b"\n")?;
                }
                None
            }
            Broadcast::Quit => {
                self.quit.store(true, std::sync::atomic::Ordering::Release);
                self.gossip_handler.thread().unpark();
                None
            }
        };

        if let Some(broadcast_reponse) = broadcast_reponse {
            let message_response = Message::new(
                message.dst,
                message.src,
                Payload::new(broadcast_reponse, message.body.msg_id),
            );
            serde_json::to_writer(&mut *writer, &message_response)?;
            writer.write_all(b"\n")?;
        }

        Ok(())
    }
}
