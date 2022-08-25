use workflow_core::enums::u32_try_from;

u32_try_from!{
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(u32)]
    pub enum RpcOps {
        Raw = 0,
        Borsh,
        Serde,
        User = 0xff,
    }
}

