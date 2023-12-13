use anyhow::Result;
use maelstrom_protocol::*;
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct EchoNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
  Echo { echo: String },
  EchoOk { echo: String },
}

impl StateMachine<Payload> for EchoNode {
  fn new(_: Init) -> Self {
    Self
  }

  fn step(&mut self, message: Message<Payload>, output: &mut MessageSender) -> Result<()> {
    match &message.body.payload {
      Payload::Echo { echo } => {
        let reply = message.into_reply(Payload::EchoOk {
          echo: echo.to_owned(),
        });
        output.send(reply)?;
      }
      Payload::EchoOk { .. } => {}
    }
    Ok(())
  }
}

fn main() -> Result<()> {
  let mut node = Node::<EchoNode, Payload>::new();

  node.run()?;

  Ok(())
}
