#![allow(dead_code)]

use std::io;

use anyhow::Ok;
use serde::{Deserialize, Serialize};

use crate::message::{self, Handler, Message, Payload};

#[derive(Debug, Serialize, Clone, Deserialize, Hash, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Echo {
    Echo { echo: String },
    EchoOk { echo: String, in_reply_to: usize },
}

#[derive(Debug, Clone)]
pub struct EchoNode {
    node: String,
}

impl EchoNode {
    pub fn new(node: String) -> EchoNode {
        EchoNode { node }
    }
}

impl Handler<Message<Echo>> for EchoNode {
    fn handle(
        &self,
        writer: &mut dyn io::Write,
        message: message::Message<Echo>,
    ) -> message::Result<()> {
        let echo_response = Echo::EchoOk {
            echo: match message.body.data {
                Echo::Echo { ref echo } => echo.clone(),
                _ => panic!("should not receive this"),
            },
            in_reply_to: message.body.msg_id.unwrap_or(1),
        };
        let message = Message::new(
            message.dst,
            message.src,
            Payload::new(echo_response, message.body.msg_id),
        );

        serde_json::to_writer(&mut *writer, &message)?;
        writer.write_all(b"\n")?;
        Ok(())
    }
}
