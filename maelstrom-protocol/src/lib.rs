use std::{
  io::{stdin, stdout, BufRead, Stderr, Stdin, StdinLock, Stdout, StdoutLock, Write},
  marker::PhantomData,
};

use anyhow::{bail, Context, Result};
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
  _data: PhantomData<(P,S)>,
}

impl<S, P> Node<S, P>
where
  S: StateMachine<P>,
{
  pub fn new() -> Self {
    Self {
      id: String::new(),
      connected_nodes: Vec::default(),
      in_channel: stdin(),
      out_channel: stdout(),
      _data: PhantomData::<(P,S)>::default(),
    }
  }
}

impl<S, P> Node<S, P>
where
  S: StateMachine<P> + StateMachine<InitPayload>,
  P: DeserializeOwned + Serialize,
{
  pub fn run(&mut self) -> Result<()> {
    let mut sender = MessageSender::new(&self.out_channel);
    let mut recver = MessageReceiver::new(&self.in_channel);

    let Some(init_msg) = recver.recv::<InitPayload>() else {
      bail!("Did not receive init message");
    };
    let init_msg: Message<InitPayload> = init_msg.context("Failed to deserialize init message")?;
    let InitPayload::Init(init) = init_msg.body.payload.clone() else {
      bail!("Received init_ok message");
    };

    let mut state = <S as StateMachine<P>>::new(init);

    state
      .step(init_msg, &mut sender)
      .context("Step function failed to process message")?;

    let inputs = recver.into_iter::<P>();
    for input in inputs {
      let input = input.context("Maelstrom input from STDIN could not be deserialized")?;
      state.step(input, &mut sender)?;
    }

    Ok(())
  }
}

pub struct MessageSender<'a> {
  chan: StdoutLock<'a>,
  id: usize,
}

impl<'a> MessageSender<'a> {
  pub(crate) fn new(chan: &'a NodeOutboundChannel) -> Self {
    let chan = chan.lock();
    Self { chan, id: 0 }
  }

  pub fn send<P>(&mut self, msg: Message<P>) -> Result<()>
  where
    P: Serialize,
  {
    let mut msg = msg;
    msg.body.id = Some(self.id);
    self.id += 1;
    serde_json::to_writer(&mut self.chan, &msg).context("serialize response message")?;
    self
      .chan
      .write_all(b"\n")
      .context("writing trailing newline")?;

    Ok(())
  }
}

struct MessageReceiver<'a> {
  chan: StdinLock<'a>,
}

impl<'a> MessageReceiver<'a> {
  pub fn new(chan: &'a NodeInboundChannel) -> Self {
    let chan = chan.lock();
    Self { chan }
  }

  pub fn recv<P>(&mut self) -> Option<Result<Message<P>>>
  where
    P: DeserializeOwned,
  {
    let mut buf = String::new();

    match self.chan.read_line(&mut buf) {
      Ok(0) => None,
      Ok(_) => {
        let msg = serde_json::from_str(buf.as_str())
          .context("Maelstrom input from STDIN could not be deserialized");
        Some(msg)
      }
      Err(e) => Some(Err(
        anyhow::Error::new(e).context("Reading input from STDIN"),
      )),
    }
  }

  pub fn into_iter<P>(self) -> MessageIterator<'a, P> {
    MessageIterator {
      receiver: self,
      _data: PhantomData::<P>::default(),
    }
  }
}

struct MessageIterator<'a, P> {
  receiver: MessageReceiver<'a>,
  _data: PhantomData<P>,
}

impl<'a, P> MessageIterator<'a, P> {
  pub fn into_inner(self) -> MessageReceiver<'a> {
    self.receiver
  }
}

impl<'a, P> Iterator for MessageIterator<'a, P>
where
  P: DeserializeOwned,
{
  type Item = Result<Message<P>>;

  fn next(&mut self) -> Option<Self::Item> {
    self.receiver.recv()
  }
}

pub trait StateMachine<P> {
  fn new(init: Init) -> Self;
  fn step(&mut self, message: Message<P>, output: &mut MessageSender) -> Result<()>;
}

impl<N> StateMachine<InitPayload> for N {
  fn new(_: Init) -> Self {
    unreachable!("State machine for init payload is not instantiable")
  }
  fn step(&mut self, message: Message<InitPayload>, output: &mut MessageSender) -> Result<()> {
    match &message.body.payload {
      InitPayload::Init(Init { .. }) => {
        let reply = message.into_reply(InitPayload::InitOk);
        output.send(reply)?;
      }
      InitPayload::InitOk => {}
    }
    Ok(())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<P> {
  pub src: String,
  #[serde(rename = "dest")]
  pub dst: String,
  pub body: Body<P>,
}

impl<P> Message<P> {
  pub fn into_reply(&self, payload: P) -> Self {
    Self {
      src: self.dst.clone(),
      dst: self.src.clone(),
      body: Body {
        id: None,
        in_reply_to: self.body.id,
        payload,
      },
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<P> {
  #[serde(rename = "msg_id")]
  pub id: Option<usize>,
  pub in_reply_to: Option<usize>,

  #[serde(flatten)]
  pub payload: P,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum InitPayload {
  Init(Init),
  InitOk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
  pub node_id: String,
  pub node_ids: Vec<String>,
}
