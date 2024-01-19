#![no_std]

extern crate alloc;
pub use crate::generated::spore::{Bytes, BytesOpt, SporeData};
use alloc::string::String;
use alloc::vec::Vec;
use molecule::prelude::{Builder, Entity};

pub mod generated;

impl Into<Bytes> for &[u8] {
    fn into(self) -> Bytes {
        let len = self.len();
        let mut vec: Vec<u8> = Vec::with_capacity(4 + len);
        vec.extend_from_slice(&(len as u32).to_le_bytes()[..]);
        vec.extend_from_slice(self);
        Bytes::new_unchecked(Bytes::from_slice(vec.as_slice()).unwrap().as_bytes())
    }
}

impl Into<BytesOpt> for &[u8] {
    fn into(self) -> BytesOpt {
        let len = self.len();
        let mut vec: Vec<u8> = Vec::with_capacity(4 + len);
        vec.extend_from_slice(&(len as u32).to_le_bytes()[..]);
        vec.extend_from_slice(self);
        BytesOpt::new_unchecked(BytesOpt::from_slice(vec.as_slice()).unwrap().as_bytes())
    }
}

#[derive(Debug, Clone)]
pub struct NativeNFTData {
    pub content_type: String,
    pub content: Vec<u8>,
    pub cluster_id: Option<Vec<u8>>,
}

impl From<NativeNFTData> for generated::spore::SporeData {
    fn from(data: NativeNFTData) -> Self {
        let content: Bytes = data.content.as_slice().into();
        let content_type: Bytes = data.content_type.as_bytes().into();
        let cluster_id = match data.cluster_id {
            Some(cluster) => BytesOpt::new_builder()
                .set(Some(cluster.as_slice().into()))
                .build(),
            None => BytesOpt::default(),
        };
        SporeData::new_builder()
            .content(content)
            .content_type(content_type)
            .cluster_id(cluster_id)
            .build()
    }
}

impl generated::spore::Bytes {
    pub fn unpack(&self) -> &[u8] {
        &self.as_slice()[4..]
    }
}
