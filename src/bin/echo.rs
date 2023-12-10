use anyhow::{bail, Result};
use maelstrom_protocol::*;
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct EchoNode {
  id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
  Echo { echo: String },
  EchoOk { echo: String },
  Init { node_id: String, node_ids: Vec<String> },
  InitOk,
}

impl StateMachine<Payload> for EchoNode {
  fn step(&mut self, message: Message<Payload>, output: &mut MessageSender) -> Result<()> {
    match message.body.payload {
      Payload::Init { .. } => {
        let reply = Message {
          src: message.dst,
          dst: message.src,
          body: Body {
            id: Some(self.id),
            in_reply_to: message.body.id,
            payload: Payload::InitOk,
          },
        };
        self.id += 1;
        output.send(reply)?;
      }
      Payload::Echo { echo } => {
        let reply = Message {
          src: message.dst,
          dst: message.src,
          body: Body {
            id: Some(self.id),
            in_reply_to: message.body.id,
            payload: Payload::EchoOk { echo },
          },
        };
        self.id += 1;
        output.send(reply)?;
      }
      Payload::InitOk { .. } => bail!("received init_ok message"),
      Payload::EchoOk { .. } => {}
    }
    Ok(())
  }
}

fn main() -> Result<()> {
  let mut node = Node::new(EchoNode::default());

  node.run()?;

  Ok(())
}
