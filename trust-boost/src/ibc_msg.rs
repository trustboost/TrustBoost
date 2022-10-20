use cosmwasm_std::{ContractResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::InputType;

/// Messages that will be sent over the IBC channel
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PacketMsg {
    MsgQueue (
        Vec<Msg>
    ),
    WhoAmI { 
        chain_id: u32,
        
    },
    // TimeoutMsg{
    //     time_view: u32
    // },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Msg {

    Request { 
        view: u32, 
        chain_id: u32 
    },
    Suggest { 
        chain_id: u32,
        view: u32,
        key2: u32,
        key2_val: InputType,
        prev_key2: i32,
        key3: u32,
        key3_val: InputType
    },
    Proof {
        key1: u32,
        key1_val: InputType,
        prev_key1: i32,
        view: u32
    },
    Abort {
        view: u32,
        chain_id: u32,
    },
    Propose { 
        chain_id: u32,
        k: u32, 
        v: InputType,
        view: u32 
    },
    Echo {
        // chain_id: u32,
        val: InputType,
        view: u32
    },
    Key1 {
        val: InputType,
        view: u32
    },
    Key2 {
        val: InputType,
        view: u32
    },
    Key3 {
        val: InputType,
        view: u32
    },
    Lock {
        val: InputType,
        view: u32
    },
    Done {
        val: InputType
    },
}

impl Msg {
    // name return the static str version of the Msg type
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Msg::Request { view: _, chain_id: _ } => stringify!(Request),
            Msg::Suggest { chain_id: _, view: _, key2: _, key2_val: _, prev_key2: _, key3: _, key3_val: _ } => stringify!(Suggest),
            Msg::Proof { key1: _, key1_val: _, prev_key1: _, view: _ } => stringify!(Proof),
            Msg::Abort { view: _, chain_id : _} => stringify!(Abort),
            Msg::Propose { chain_id: _, k: _, v: _, view: _ } => stringify!(Propose),
            Msg::Echo { val: _, view : _} => stringify!(Echo),
            Msg::Key1 { val: _, view : _} => stringify!(Key1),
            Msg::Key2 { val: _, view : _} => stringify!(Key2),
            Msg::Key3 { val: _, view : _} => stringify!(Key3),
            Msg::Lock { val: _, view : _} => stringify!(Lock),
            Msg::Done { val: _ } => stringify!(Done),
        }
    }
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// #[serde(rename_all = "snake_case")]
// pub struct MsgQueue {
//     q: Vec<PacketMsg>
// }

/// All IBC acknowledgements are wrapped in `ContractResult`.
/// The success value depends on the PacketMsg variant.
pub type AcknowledgementMsg<T> = ContractResult<T>;

/// This is the success response we send on ack for PacketMsg::Dispatch.
/// Just acknowledge success or error
// pub type DispatchResponse = ();

/// This is the success response we send on ack for PacketMsg::WhoAmI.
/// Return the caller's account address on the remote chain
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhoAmIResponse {
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposeResponse { 
    // pub tx_id: u32
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CommitResponse {
    pub tx_id: u32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RequestResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SuggestResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProofResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EchoResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key1Response {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key2Response {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key3Response {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DoneResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MsgQueueResponse {
}
