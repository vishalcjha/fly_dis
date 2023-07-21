use std::str::FromStr;

use anyhow::Ok;
use serde::{Deserialize, Serialize};

use crate::message::{self, Handler, Message, ParseError, Payload};

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
impl FromStr for Message<Echo> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(&s)?)
    }
}

impl Handler<Echo> for EchoNode {
    fn handle(&self, message: &message::Message<Echo>) -> message::Result<Message<Echo>> {
        let echo_response = Echo::EchoOk {
            echo: match message.body.data {
                Echo::Echo { ref echo } => echo.clone(),
                _ => panic!("should not receive this"),
            },
            in_reply_to: message.body.msg_id.unwrap_or(1),
        };
        let message = Message::new(
            message.dst.clone(),
            message.src.clone(),
            Payload::new(echo_response, message.body.msg_id),
        );

        Ok(message)
    }
}
