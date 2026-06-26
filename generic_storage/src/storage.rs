use crate::error::{Result, StorageError};
use crate::serializers::{Borsh, Json, Wincode};
use std::marker::PhantomData;

pub struct Storage<T, S> {
    bytes: Option<Vec<u8>>,
    serializer: S,
    _marker: PhantomData<T>,
}

//borsh
impl<T> Storage<T, Borsh>
where
    T: borsh::BorshSerialize + borsh::BorshDeserialize,
{
    pub fn new() -> Self {
        Self {
            bytes: None,
            serializer: Borsh,
            _marker: PhantomData,
        }
    }
    pub fn save(&mut self, value: &T) -> Result<()> {
        self.bytes = Some(self.serializer.to_bytes(value)?);
        Ok(())
    }
    pub fn load(&self) -> Result<T> {
        let bytes = self.bytes.as_deref().ok_or(StorageError::Empty)?;
        self.serializer.from_bytes(bytes)
    }
    pub fn has_data(&self) -> bool {
        self.bytes.is_some()
    }
    pub fn byte_len(&self) -> Option<usize> {
        self.bytes.as_ref().map(Vec::len)
    }
}

impl<T: borsh::BorshSerialize + borsh::BorshDeserialize> Default for Storage<T, Borsh> {
    fn default() -> Self {
        Self::new()
    }
}

//wincode
impl<T> Storage<T, Wincode>
where
    T: wincode::SchemaWrite<wincode::config::DefaultConfig, Src = T>
        + wincode::DeserializeOwned<Dst = T>,
{
    pub fn new() -> Self {
        Self {
            bytes: None,
            serializer: Wincode,
            _marker: PhantomData,
        }
    }
    pub fn save(&mut self, value: &T) -> Result<()> {
        self.bytes = Some(self.serializer.to_bytes(value)?);
        Ok(())
    }
    pub fn load(&self) -> Result<T> {
        let bytes = self.bytes.as_deref().ok_or(StorageError::Empty)?;
        self.serializer.from_bytes(bytes)
    }
    pub fn has_data(&self) -> bool {
        self.bytes.is_some()
    }
    pub fn byte_len(&self) -> Option<usize> {
        self.bytes.as_ref().map(Vec::len)
    }
}

impl<
        T: wincode::SchemaWrite<wincode::config::DefaultConfig, Src = T>
            + wincode::DeserializeOwned<Dst = T>,
    > Default for Storage<T, Wincode>
{
    fn default() -> Self {
        Self::new()
    }
}

//json
impl<T> Storage<T, Json>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    pub fn new() -> Self {
        Self {
            bytes: None,
            serializer: Json,
            _marker: PhantomData,
        }
    }
    pub fn save(&mut self, value: &T) -> Result<()> {
        self.bytes = Some(self.serializer.to_bytes(value)?);
        Ok(())
    }
    pub fn load(&self) -> Result<T> {
        let bytes = self.bytes.as_deref().ok_or(StorageError::Empty)?;
        self.serializer.from_bytes(bytes)
    }
    pub fn has_data(&self) -> bool {
        self.bytes.is_some()
    }
    pub fn byte_len(&self) -> Option<usize> {
        self.bytes.as_ref().map(Vec::len)
    }
    pub fn as_json_str(&self) -> Option<&str> {
        self.bytes
            .as_deref()
            .and_then(|b| std::str::from_utf8(b).ok())
    }
}

impl<T: serde::Serialize + serde::de::DeserializeOwned> Default for Storage<T, Json> {
    fn default() -> Self {
        Self::new()
    }
}

//transcription
pub fn transcribe_borsh_to_json<T>(
    src: &Storage<T, Borsh>,
    dst: &mut Storage<T, Json>,
) -> Result<()>
where
    T: borsh::BorshSerialize
        + borsh::BorshDeserialize
        + serde::Serialize
        + serde::de::DeserializeOwned,
{
    dst.save(&src.load()?)
}

pub fn transcribe_json_to_wincode<T>(
    src: &Storage<T, Json>,
    dst: &mut Storage<T, Wincode>,
) -> Result<()>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + wincode::SchemaWrite<wincode::config::DefaultConfig, Src = T>
        + wincode::DeserializeOwned<Dst = T>,
{
    dst.save(&src.load()?)
}

pub fn transcribe_wincode_to_borsh<T>(
    src: &Storage<T, Wincode>,
    dst: &mut Storage<T, Borsh>,
) -> Result<()>
where
    T: wincode::SchemaWrite<wincode::config::DefaultConfig, Src = T>
        + wincode::DeserializeOwned<Dst = T>
        + borsh::BorshSerialize
        + borsh::BorshDeserialize,
{
    dst.save(&src.load()?)
}
