use anyhow::Result;
use maelstrom_protocol::*;
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct UniqueIdsNode {
  node: String,
  id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
  Generate,
  GenerateOk {
    #[serde(rename = "id")]
    guid: String,
  },
}

impl StateMachine<Payload> for UniqueIdsNode {
  fn new(init: Init) -> Self {
    Self {
      node: init.node_id,
      id: 1,
    }
  }

  fn step(&mut self, message: Message<Payload>, output: &mut MessageSender) -> Result<()> {
    match message.body.payload {
      Payload::Generate { .. } => {
        let guid = self.next().expect("Next GUID");
        let reply = message.into_reply(Payload::GenerateOk { guid });
        output.send(reply)?;
      }
      Payload::GenerateOk { .. } => {}
    }
    Ok(())
  }
}

impl Iterator for UniqueIdsNode {
  type Item = String;

  fn next(&mut self) -> Option<Self::Item> {
    let guid = format!("{}-{}", self.node, self.id);
    self.id += 1;

    Some(guid)
  }
}

fn main() -> Result<()> {
  let mut node = Node::<UniqueIdsNode, Payload>::new();

  node.run()?;

  Ok(())
}
