#![allow(dead_code, unused_variables)]
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    io::BufRead,
    sync::mpsc::{channel, Sender},
    thread,
    time::Duration,
};

use anyhow::Ok;
use serde::{Deserialize, Serialize};

use crate::{
    message::{self, Handler, Init, Message, Payload},
    periodic_thread::PeriodicThread,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Counter {
    Add { delta: usize },
    AddOk { in_reply_to: usize },
    Read,
    ReadOk { value: usize, in_reply_to: usize },
    Current { value: usize },
}

#[derive(Debug, Clone)]
pub enum Internal {
    TriggerDispatch,
    TerminateDispatcher,
}

#[derive(Debug, Clone)]
pub enum ExternalInternal {
    External(Message<Counter>),
    Internal(Internal),
}

#[derive(Debug)]
pub struct CounterNode {
    node_id: String,
    all_node_ids: Vec<String>,
    current_count: Cell<usize>,
    other_node_count_map: RefCell<HashMap<String, usize>>,
    gossip_trigger_task: RefCell<Option<PeriodicThread>>,
}

impl CounterNode {
    pub fn new(node_id: String, all_node_ids: Vec<String>, tx: Sender<ExternalInternal>) -> Self {
        let gossip_trigger_task = PeriodicThread::new(
            move || {
                tx.send(ExternalInternal::Internal(Internal::TriggerDispatch {}))?;
                Ok(())
            },
            Duration::from_secs(1),
        );
        let other_node_count_map = all_node_ids
            .iter()
            .filter(|id| node_id != **id)
            .map(|node_id| (node_id.clone(), 0))
            .collect();
        CounterNode {
            node_id,
            all_node_ids: all_node_ids,
            current_count: Cell::new(0),
            other_node_count_map: RefCell::new(other_node_count_map),
            gossip_trigger_task: RefCell::new(Some(gossip_trigger_task)),
        }
    }

    pub fn main_loop() -> message::Result<()> {
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
                let incoming = line?.parse::<Message<Counter>>()?;
                tx_cloned.send(ExternalInternal::External(incoming))?;
            }
            tx_cloned.send(ExternalInternal::Internal(Internal::TerminateDispatcher))?;
            Ok(())
        });

        let (node_id, all_nodes_ids) = match init_message.body.data {
            Init::Init { node_id, node_ids } => (node_id.clone(), node_ids),
            _ => panic!("First message should be of type echo"),
        };
        let counter_node = CounterNode::new(node_id, all_nodes_ids, tx);

        for message in rx {
            counter_node.handle(&mut stdout, message)?;
        }

        let _ = join_handler.join().expect("Main panicked");

        Ok(())
    }
}

trait ToResponse<T> {
    fn to_response(from: Message<T>, to: T) -> Message<T>;
}

impl<T> ToResponse<T> for Message<T> {
    fn to_response(from: Message<T>, to: T) -> Message<T> {
        Message {
            src: from.dst,
            dst: from.src,
            body: Payload::new(to, from.body.msg_id),
        }
    }
}
impl Handler<ExternalInternal> for CounterNode {
    fn handle(
        &self,
        writer: &mut dyn std::io::Write,
        message: ExternalInternal,
    ) -> message::Result<()> {
        let maybe_response = match message {
            ExternalInternal::External(message) => match message.body.data {
                Counter::Add { delta } => {
                    let updated_value = self.current_count.get() + delta;
                    self.current_count.set(updated_value);
                    let in_reply_to = message.body.msg_id.unwrap_or(1);
                    Some(Message::to_response(
                        message,
                        Counter::AddOk { in_reply_to },
                    ))
                }
                Counter::AddOk { .. } => None,
                Counter::Read => {
                    let total_count = self.other_node_count_map.borrow().values().sum::<usize>();
                    let in_reply_to = message.body.msg_id.unwrap_or(1);
                    Some(Message::to_response(
                        message,
                        Counter::ReadOk {
                            value: total_count + self.current_count.get(),
                            in_reply_to,
                        },
                    ))
                }
                Counter::ReadOk { .. } => None,
                Counter::Current { value } => {
                    // println!(
                    //     "Received {} for node {} with map {:?}",
                    //     value,
                    //     self.node_id,
                    //     self.other_node_count_map.borrow()
                    // );
                    let from = message.src;
                    self.other_node_count_map
                        .borrow_mut()
                        .entry(from)
                        .and_modify(|count| *count = value);
                    None
                }
            },
            ExternalInternal::Internal(message) => match message {
                Internal::TerminateDispatcher => {
                    if let Some(handler) = self.gossip_trigger_task.take() {
                        println!("requesting close of thread");
                        drop(handler);
                    }
                    None
                }
                Internal::TriggerDispatch => {
                    let current_message = Counter::Current {
                        value: self.current_count.get(),
                    };
                    for other in self.all_node_ids.iter() {
                        if *other == self.node_id {
                            continue;
                        }
                        let current_message = Message::new(
                            self.node_id.clone(),
                            other.clone(),
                            Payload {
                                data: current_message.clone(),
                                msg_id: None,
                            },
                        );
                        serde_json::to_writer(&mut *writer, &current_message)?;
                        writer.write_all(b"\n")?;
                    }
                    None
                }
            },
        };
        if let Some(response) = maybe_response {
            serde_json::to_writer(&mut *writer, &response)?;
            writer.write_all(b"\n")?;
        }
        Ok(())
    }
}
