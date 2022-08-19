use workflow_core::enums::u32_try_from;

pub mod client;
pub mod server;
pub mod message;
pub mod error;

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct ReqHeader {
    id : u64,
    op : u32,
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct RespHeader {
    id : u64,
    status : u32
}

u32_try_from! {
    pub enum RespStatus {
        Success = 0,
        Error = 1,
    }
}

u32_try_from!{
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(u32)]
    pub enum RpcOps {
        Raw = 0,
        Borsh,
    }
}

