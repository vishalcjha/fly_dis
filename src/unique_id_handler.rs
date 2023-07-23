#![allow(dead_code)]

use std::{cell::Cell, io::Write};

use anyhow::Ok;
use serde::{Deserialize, Serialize};

use crate::message::{self, Handler, Message, Payload};

#[derive(Debug, Serialize, Clone, Deserialize, Hash, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Generate {
    Generate,
    GenerateOk { id: String, in_reply_to: usize },
}

#[derive(Debug, Clone)]
pub struct UniqueIdNode {
    node_id: String,
    processed_id_count: Cell<usize>,
}

impl UniqueIdNode {
    pub fn new(node_id: String) -> Self {
        UniqueIdNode {
            node_id,
            processed_id_count: Cell::new(0),
        }
    }
}

impl Handler<Message<Generate>> for UniqueIdNode {
    fn handle(&self, writer: &mut dyn Write, message: Message<Generate>) -> message::Result<()> {
        let generate_response = match message.body.data {
            Generate::Generate => {
                let current_processed_id = self.processed_id_count.get();
                self.processed_id_count.set(current_processed_id + 1);
                let gen_ok = Generate::GenerateOk {
                    id: format!("{}_{}", self.node_id, current_processed_id + 1),
                    in_reply_to: message.body.msg_id.unwrap_or(1),
                };
                message::Message {
                    src: message.dst,
                    dst: message.src,
                    body: Payload::new(gen_ok, message.body.msg_id),
                }
            }
            _ => message,
        };

        serde_json::to_writer(&mut *writer, &generate_response)?;
        writer.write_all(b"\n")?;
        Ok(())
    }
}
