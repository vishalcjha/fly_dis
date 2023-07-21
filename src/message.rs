#![allow(dead_code, unused_variables)]

use anyhow::Ok;
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, anyhow::Error>;
pub struct ParseError(String);
#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct Message<T> {
    pub src: String,
    #[serde(rename = "dest")]
    pub dst: String,
    pub body: Payload<T>,
}

impl<T> Message<T> {
    pub fn new(src: String, dst: String, body: Payload<T>) -> Self {
        Message { src, dst, body }
    }
}

pub trait Handler<T> {
    fn handle(&self, message: &Message<T>) -> Result<Message<T>>;
}

impl Handler<Init> for Init {
    fn handle(&self, message: &Message<Init>) -> Result<Message<Init>> {
        let init_ok = Init::InitOk {
            in_reply_to: message.body.msg_id.unwrap_or(1),
        };
        let message = Message::new(
            message.dst.clone(),
            message.src.clone(),
            Payload {
                data: init_ok,
                msg_id: None,
            },
        );
        Ok(message)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct Payload<T> {
    #[serde(flatten)]
    pub data: T,
    pub msg_id: Option<usize>,
}

impl<T> Payload<T> {
    pub fn new(data: T, msg_id: Option<usize>) -> Self {
        Payload { data, msg_id }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Init {
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk {
        in_reply_to: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_message_serialization() {
        let init_message = Init::Init {
            node_id: "n3".to_string(),
            node_ids: vec!["n1".to_string(), "n2".to_string(), "n3".to_string()],
        };
        let serde_init_message = serde_json::to_string(&init_message).unwrap();
        println!("{}", serde_init_message);
        assert_eq!(
            r#"{"type":"init","node_id":"n3","node_ids":["n1","n2","n3"]}"#,
            serde_init_message
        );

        let message = Message::<Init>::new(
            "n1".to_string(),
            "c2".to_string(),
            Payload::new(init_message),
        );

        let serde_message_with_body = serde_json::to_string(&message).unwrap();
        println!("{}", serde_message_with_body);
        assert_eq!(
            r#"{"src":"n1","dest":"c2","body":{"type":"init","node_id":"n3","node_ids":["n1","n2","n3"],"msg_id":null}}"#,
            serde_message_with_body
        );
    }
}
