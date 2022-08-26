use std::mem::size_of;
use workflow_websocket::client::message::Message as WebSocketMessage;
use crate::asynchronous::client::error::Error;
use borsh::BorshDeserialize;
use workflow_core::enums::u32_try_from;

pub enum Message<'data> {
    Request(&'data [u8]),
    Post(&'data [u8]),
}

impl<'data> Message<'data> {
    fn data(&self) -> &'data [u8] {
        match self {
            Message::Request(data) => data,
            Message::Post(data) => data,
        }
    }
}

pub fn to_ws_msg(msg : (ReqHeader, Message<'_>)) -> WebSocketMessage {
    let (header, message) = msg;
    let data = message.data();
    let len = data.len() + size_of::<ReqHeader>();
    let mut buffer = Vec::with_capacity(len);
    unsafe { buffer.set_len(len); }
    let dest_header: &mut ReqHeader = unsafe { std::mem::transmute(&mut buffer[0]) };
    *dest_header = header;
    buffer[size_of::<ReqHeader>()..].copy_from_slice(data);
    buffer.into()
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct ReqHeader {
    pub id : u64,
    pub op : u32,
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct RespHeader {
    pub id : u64,
    pub status : u32
}

u32_try_from! {
    pub enum RespStatus {
        Success = 0,
        Error = 1,
    }
}

#[derive(Debug)]
pub enum RespError<T>
where
    T : BorshDeserialize
{
    NoData,
    Data(T),
    Rpc(Error),
}

#[derive(Debug)]
pub struct ReqMessage<'data> {
    pub id : u64,
    pub op : u32,
    pub data : &'data [u8],
    // pub data : Option<&'data [u8]>,
    // pub data : Option<&'data [u8]>,
}

impl<'data> TryFrom<&'data Vec<u8>> for ReqMessage<'data> {
    type Error = Error;

    fn try_from(src: &'data Vec<u8>) -> Result<Self, Self::Error> {
        let v : ReqMessage = src[..].try_into()?;
        Ok(v)
    }
}

impl<'data> TryFrom<&'data [u8]> for ReqMessage<'data> {
    type Error = Error;

    fn try_from(src: &'data [u8]) -> Result<Self, Self::Error> {
        if src.len() < size_of::<ReqHeader>() {
            return Err(Error::HeaderSize);
        }

        let header: &ReqHeader = unsafe { std::mem::transmute(&src[0]) };
        let ReqHeader { id, op } = *header;
        // let data = if src.len() == size_of::<ReqHeader>() { None } else { Some(&src[size_of::<ReqHeader>()..]) };
        let data = &src[size_of::<ReqHeader>()..];

        let message = ReqMessage {
            id,
            op,
            data
        };
        
        Ok(message)
    }
}




#[derive(Debug)]
pub struct RespMessage<'data> {
    pub id : u64,
    pub status : u32,
    // pub data : Option<&'data [u8]>,
    pub data : &'data [u8],
}

impl<'data> RespMessage<'data> {
    pub fn new(id: u64, status: u32, data: &'data [u8]) -> RespMessage<'data> {
        RespMessage {
            id,
            status,
            data
        }
    }

    pub fn try_to_vec(&self) -> Result<Vec<u8>, Error> {
        // match self.data {
        //     Some(data) => {
                let len = size_of::<RespHeader>() + self.data.len();
                let mut buffer = Vec::with_capacity(len);
                unsafe { buffer.set_len(len); }
                let header: &mut RespHeader = unsafe { std::mem::transmute(&mut buffer[0]) };
                *header = RespHeader { id : self.id, status : self.status };
                if self.data.len() > 0 {
                    buffer[size_of::<RespHeader>()..].copy_from_slice(&self.data);
                }
                Ok(buffer)
            // },
            // None => {
            //     let mut buffer = Vec::with_capacity(size_of::<RespHeader>());
            //     unsafe { buffer.set_len(size_of::<RespHeader>()); }
            //     let header: &mut RespHeader = unsafe { std::mem::transmute(&mut buffer[0]) };
            //     *header = RespHeader { id : self.id, status : self.status };
            //     Ok(buffer)

            // }
        // }
    }
}

impl<'data> TryFrom<&'data [u8]> for RespMessage<'data> {
    type Error = Error;

    fn try_from(src: &'data [u8]) -> Result<Self, Self::Error> {
        if src.len() < size_of::<ReqHeader>() {
            return Err(Error::HeaderSize);
        }

        let header: &RespHeader = unsafe { std::mem::transmute(&src[0]) };
        let RespHeader { id, status } = *header;
        // let data = if src.len() == size_of::<RespHeader>() { None } else { Some(&src[size_of::<RespHeader>()..]) };
        let data = &src[size_of::<RespHeader>()..];

        let message = RespMessage {
            id,
            status,
            data
        };
        
        Ok(message)
    }
}
