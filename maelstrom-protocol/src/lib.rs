use std::{
  io::{stdin, stdout, Stderr, Stdin, Stdout, StdoutLock, Write},
  marker::PhantomData,
};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub type NodeOutboundChannel = Stdout;
pub type NodeInboundChannel = Stdin;
pub type NodeLogChannel = Stderr;
pub type MessageWriter<'a> = serde_json::Serializer<StdoutLock<'a>>;

pub struct Node<S, P>
where
  S: StateMachine<P>,
{
  pub id: String,
  pub connected_nodes: Vec<String>,
  pub in_channel: NodeInboundChannel,
  pub out_channel: NodeOutboundChannel,
  pub state: S,
  _data: PhantomData<P>,
}

impl<S, P> Node<S, P>
where
  S: StateMachine<P>,
  P: DeserializeOwned + Serialize,
{
  pub fn new(state: S) -> Self {
    Self {
      id: String::new(),
      connected_nodes: Vec::default(),
      in_channel: stdin(),
      out_channel: stdout(),
      state,
      _data: PhantomData::<P>::default(),
    }
  }

  pub fn run(&mut self) -> Result<()> {
    let in_chan = self.in_channel.lock();
    let inputs = serde_json::Deserializer::from_reader(in_chan).into_iter::<Message<P>>();
    let mut outputs = MessageSender::new(&self.out_channel);

    for input in inputs {
      let input = input.context("Maelstrom input from STDIN could not be deserialized")?;
      self.state.step(input, &mut outputs)?;
    }
    Ok(())
  }
}

pub struct MessageSender<'a> {
  chan: StdoutLock<'a>,
}

impl<'a> MessageSender<'a> {
  pub(crate) fn new(chan: &'a NodeOutboundChannel) -> Self {
    let chan = chan.lock();
    Self { chan }
  }

  pub fn send<P>(&mut self, msg: Message<P>) -> Result<()>
  where
    P: Serialize,
  {
    serde_json::to_writer(&mut self.chan, &msg).context("serialize response message")?;
    self
      .chan
      .write_all(b"\n")
      .context("writing trailing newline")?;

    Ok(())
  }
}

pub trait StateMachine<P> {
  fn step(&mut self, message: Message<P>, output: &mut MessageSender) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<P> {
  pub src: String,
  #[serde(rename = "dest")]
  pub dst: String,
  pub body: Body<P>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<P> {
  #[serde(rename = "msg_id")]
  pub id: Option<usize>,
  pub in_reply_to: Option<usize>,

  #[serde(flatten)]
  payload: P,
}

impl<P> Body<P> {
  pub fn new(payload: P) -> Self {
    Self {
      payload,
      id: None,
      in_reply_to: None,
    }
  }

  pub fn set_msg_id(&mut self, id: usize) -> &mut Self {
    self.id = Some(id);
    self
  }

  pub fn set_in_reply_to(&mut self, irt: usize) -> &mut Self {
    self.in_reply_to = Some(irt);
    self
  }
}
