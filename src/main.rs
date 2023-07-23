#![allow(unused_imports)]
use std::{
    io::{BufRead, Write},
    sync::mpsc::channel,
    thread,
};

use anyhow::Ok;
use broadcase_handler::{Broadcast, BroadcastNode};
use counter::{Counter, CounterNode};
use echo_handler::{Echo, EchoNode};
use message::{Handler, Init, Message, Payload};
use unique_id_handler::{Generate, UniqueIdNode};

mod broadcase_handler;
mod counter;
mod echo_handler;
mod message;
mod periodic_thread;
mod unique_id_handler;
fn main() -> message::Result<()> {
    CounterNode::main_loop()?;
    Ok(())
}
