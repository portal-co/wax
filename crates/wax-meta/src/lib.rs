#![no_std]
//! Canonical v0.1 WASM semantic metadata manifests.
//!
//! A manifest is source-neutral: it may be carried by a WASM custom section or
//! supplied by an embedding context.  Whether a consumer acts on it is an
//! explicit [`MetadataMode`] decision.

extern crate alloc;

use alloc::{collections::BTreeMap, string::{String, ToString}, vec, vec::Vec};
use core::fmt;
use sha3::{Digest as _, Sha3_256};

pub const SECTION_NAME: &str = "portal.wasm.meta.v1";
pub const MAGIC: [u8; 4] = *b"WSMM";
pub const VERSION: u8 = 1;
const HEADER_LEN: usize = 6;

/// Bounds used while decoding untrusted manifest bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_payload: usize,
    pub max_entries: usize,
    pub max_key_len: usize,
    pub max_value_len: usize,
    pub max_depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_payload: 1024 * 1024,
            max_entries: 4096,
            max_key_len: 256,
            max_value_len: 512 * 1024,
            max_depth: 32,
        }
    }
}

/// The typed values supported by the v0.1 canonical map.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    U64(u64),
    I64(i64),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Digest([u8; 32]),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

/// A source-neutral WSMM v0.1 key-value map.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    entries: BTreeMap<String, Value>,
}

impl Manifest {
    pub fn new() -> Self { Self::default() }

    pub fn entries(&self) -> &BTreeMap<String, Value> { &self.entries }

    pub fn get(&self, key: &str) -> Option<&Value> { self.entries.get(key) }

    /// Inserts a canonical key. Invalid keys are rejected before they enter the map.
    pub fn insert(&mut self, key: String, value: Value) -> Result<Option<Value>, Error> {
        validate_key(&key)?;
        Ok(self.entries.insert(key, value))
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> { self.entries.remove(key) }

    /// Encodes the deterministic WSMM payload (not a WASM section envelope).
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC);
        out.push(VERSION);
        out.push(0); // reserved flags
        put_u64(&mut out, self.entries.len() as u64);
        encode_entries(&mut out, &self.entries)?;
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> { Self::decode_with_limits(bytes, Limits::default()) }

    pub fn decode_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        if bytes.len() > limits.max_payload { return Err(Error::Limit("payload")); }
        if bytes.len() < HEADER_LEN || bytes[..4] != MAGIC { return Err(Error::Magic); }
        if bytes[4] != VERSION { return Err(Error::Version(bytes[4])); }
        if bytes[5] != 0 { return Err(Error::Flags(bytes[5])); }
        let mut decoder = Decoder { bytes, pos: HEADER_LEN, limits };
        let count = decoder.u64()?;
        if count as usize > limits.max_entries { return Err(Error::Limit("entries")); }
        let entries = decoder.entries(count as usize, 0)?;
        if decoder.pos != bytes.len() { return Err(Error::TrailingBytes); }
        Ok(Self { entries })
    }

    /// Canonical bytes suitable for the `hash.semantic` binding value.
    pub fn semantic_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut copy = self.clone();
        copy.entries.retain(|key, _| !key.starts_with("hash."));
        copy.encode()
    }

    pub fn semantic_hash(&self) -> Result<[u8; 32], Error> {
        Ok(hash_domain(b"portal.wsmm.semantic.v1", &self.semantic_bytes()?))
    }

    pub fn snapshot(&self) -> Result<MetadataSnapshot, Error> {
        Ok(MetadataSnapshot(self.semantic_hash()?))
    }
}

/// Stable cache identity for the manifest a lazy transformation reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetadataSnapshot(pub [u8; 32]);

/// Consumers must opt in to semantic effects. `RespectUnstable` is intentionally
/// source-neutral and does not authenticate a manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataMode {
    Ignore,
    RespectUnstable,
    RequireSignature,
}

/// Context capability used by Wax lazy transforms and whole-module adapters.
pub trait WasmMetadataContext {
    fn metadata_snapshot(&self) -> Option<MetadataSnapshot>;
    fn metadata(&self) -> Option<&Manifest>;
    fn metadata_mode(&self) -> MetadataMode;
}

pub trait WasmMetadataMutContext: WasmMetadataContext {
    fn metadata_mut(&mut self) -> Option<&mut Manifest>;
}

/// A simple owned Context implementation for embedders and tests.
#[derive(Clone, Debug, Default)]
pub struct MetadataContext {
    pub manifest: Option<Manifest>,
    pub mode: MetadataMode,
}

impl Default for MetadataMode {
    fn default() -> Self { Self::Ignore }
}

impl WasmMetadataContext for MetadataContext {
    fn metadata_snapshot(&self) -> Option<MetadataSnapshot> {
        self.manifest.as_ref().and_then(|m| m.snapshot().ok())
    }
    fn metadata(&self) -> Option<&Manifest> { self.manifest.as_ref() }
    fn metadata_mode(&self) -> MetadataMode { self.mode }
}

impl WasmMetadataMutContext for MetadataContext {
    fn metadata_mut(&mut self) -> Option<&mut Manifest> { self.manifest.as_mut() }
}

/// Produces a domain-separated SHA3-256 digest used by v0.1 bindings.
pub fn hash_domain(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha3_256::new();
    hash.update(domain);
    hash.update([0]);
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
    hash.finalize().into()
}

/// Hashes a canonical code-body sequence supplied by a module adapter.
pub fn code_hash(canonical_bodies: &[u8]) -> [u8; 32] {
    hash_domain(b"portal.wsmm.code.v1", canonical_bodies)
}

/// Hashes a canonical data-segment sequence supplied by a module adapter.
pub fn data_hash(canonical_segments: &[u8]) -> [u8; 32] {
    hash_domain(b"portal.wsmm.data.v1", canonical_segments)
}

/// Hashes a canonical interface sequence, including table element segments.
pub fn interface_hash(canonical_interface: &[u8]) -> [u8; 32] {
    hash_domain(b"portal.wsmm.interface.v1", canonical_interface)
}

pub fn is_standard_key(key: &str) -> bool {
    matches!(key, "format.version" | "hash.algorithm" | "hash.code" | "hash.data" | "hash.interface" | "hash.semantic" | "memory.count" | "abi.name" | "abi.version" | "abi.entrypoints" | "semantics.deterministic" | "semantics.traps")
        || key.starts_with("memory/")
        || key.starts_with("data/")
        || key.starts_with("table/")
        || key.starts_with("semantic/")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Magic,
    Version(u8),
    Flags(u8),
    Eof,
    Leb,
    NonCanonicalLeb,
    Utf8,
    InvalidKey,
    DuplicateKey,
    InvalidBool,
    UnknownTag(u8),
    Limit(&'static str),
    TrailingBytes,
    InvalidValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid WSMM manifest: {self:?}")
    }
}

const U64: u8 = 0;
const I64: u8 = 1;
const BOOL: u8 = 2;
const STRING: u8 = 3;
const BYTES: u8 = 4;
const DIGEST: u8 = 5;
const LIST: u8 = 6;
const MAP: u8 = 7;

fn validate_key(key: &str) -> Result<(), Error> {
    let bytes = key.as_bytes();
    if bytes.is_empty() { return Err(Error::InvalidKey); }
    for (i, &b) in bytes.iter().enumerate() {
        let first = i == 0;
        let allowed = b.is_ascii_lowercase() || b.is_ascii_digit() || (!first && matches!(b, b'.' | b'_' | b'/' | b'-'));
        if !allowed { return Err(Error::InvalidKey); }
    }
    Ok(())
}

fn encode_entries(out: &mut Vec<u8>, entries: &BTreeMap<String, Value>) -> Result<(), Error> {
    for (key, value) in entries {
        validate_key(key)?;
        put_u64(out, key.len() as u64);
        out.extend_from_slice(key.as_bytes());
        encode_value(out, value)?;
    }
    Ok(())
}

fn encode_value(out: &mut Vec<u8>, value: &Value) -> Result<(), Error> {
    let (tag, payload) = match value {
        Value::U64(v) => { let mut p = Vec::new(); put_u64(&mut p, *v); (U64, p) }
        Value::I64(v) => (I64, v.to_le_bytes().to_vec()),
        Value::Bool(v) => (BOOL, vec![u8::from(*v)]),
        Value::String(v) => (STRING, v.as_bytes().to_vec()),
        Value::Bytes(v) => (BYTES, v.clone()),
        Value::Digest(v) => (DIGEST, v.to_vec()),
        Value::List(values) => {
            let mut p = Vec::new(); put_u64(&mut p, values.len() as u64);
            for v in values { encode_value(&mut p, v)?; }
            (LIST, p)
        }
        Value::Map(entries) => {
            let mut p = Vec::new(); put_u64(&mut p, entries.len() as u64); encode_entries(&mut p, entries)?; (MAP, p)
        }
    };
    out.push(tag);
    put_u64(out, payload.len() as u64);
    out.extend_from_slice(&payload);
    Ok(())
}

struct Decoder<'a> { bytes: &'a [u8], pos: usize, limits: Limits }
impl<'a> Decoder<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(len).ok_or(Error::Limit("length"))?;
        let result = self.bytes.get(self.pos..end).ok_or(Error::Eof)?;
        self.pos = end;
        Ok(result)
    }
    fn u64(&mut self) -> Result<u64, Error> {
        let start = self.pos;
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = *self.take(1)?.first().ok_or(Error::Eof)?;
            if shift == 63 && byte > 1 { return Err(Error::Leb); }
            value |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                let mut canonical = Vec::new(); put_u64(&mut canonical, value);
                if self.bytes[start..self.pos] != canonical { return Err(Error::NonCanonicalLeb); }
                return Ok(value);
            }
        }
        Err(Error::Leb)
    }
    fn entries(&mut self, count: usize, depth: usize) -> Result<BTreeMap<String, Value>, Error> {
        if depth > self.limits.max_depth { return Err(Error::Limit("depth")); }
        let mut entries = BTreeMap::new();
        let mut previous: Option<Vec<u8>> = None;
        for _ in 0..count {
            let len = self.u64()? as usize;
            if len > self.limits.max_key_len { return Err(Error::Limit("key")); }
            let raw = self.take(len)?;
            if let Some(prev) = previous.as_ref() { if raw <= &prev[..] { return Err(Error::DuplicateKey); } }
            let key: String = core::str::from_utf8(raw).map_err(|_| Error::Utf8)?.to_string();
            validate_key(&key)?;
            previous = Some(raw.to_vec());
            let value = self.value(depth + 1)?;
            if entries.insert(key, value).is_some() { return Err(Error::DuplicateKey); }
        }
        Ok(entries)
    }
    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > self.limits.max_depth { return Err(Error::Limit("depth")); }
        let tag = *self.take(1)?.first().ok_or(Error::Eof)?;
        let len = self.u64()? as usize;
        if len > self.limits.max_value_len { return Err(Error::Limit("value")); }
        let payload = self.take(len)?;
        let mut d = Decoder { bytes: payload, pos: 0, limits: self.limits };
        let value = match tag {
            U64 => Value::U64(d.u64()?),
            I64 => { let b = d.take(8)?; Value::I64(i64::from_le_bytes(b.try_into().map_err(|_| Error::InvalidValue)?)) }
            BOOL => match d.take(1)? { [0] => Value::Bool(false), [1] => Value::Bool(true), _ => return Err(Error::InvalidBool) },
            STRING => Value::String(core::str::from_utf8(payload).map_err(|_| Error::Utf8)?.into()),
            BYTES => Value::Bytes(payload.to_vec()),
            DIGEST => Value::Digest(payload.try_into().map_err(|_| Error::InvalidValue)?),
            LIST => {
                let count = d.u64()? as usize;
                if count > d.limits.max_entries { return Err(Error::Limit("list")); }
                let mut values = Vec::with_capacity(count);
                for _ in 0..count { values.push(d.value(depth + 1)?); }
                Value::List(values)
            }
            MAP => {
                let count = d.u64()? as usize;
                if count > d.limits.max_entries { return Err(Error::Limit("map")); }
                Value::Map(d.entries(count, depth + 1)?)
            }
            _ => return Err(Error::UnknownTag(tag)),
        };
        if d.pos != payload.len() { return Err(Error::TrailingBytes); }
        Ok(value)
    }
}

fn put_u64(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 { byte |= 0x80; }
        out.push(byte);
        if value == 0 { return; }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    #[test]
    fn canonical_round_trip_and_snapshot() {
        let mut manifest = Manifest::new();
        manifest.insert("memory/0/maximum_pages".into(), Value::U64(1)).unwrap();
        manifest.insert("table/0/indirect_targets".into(), Value::List(vec![Value::U64(3)])).unwrap();
        let bytes = manifest.encode().unwrap();
        assert_eq!(Manifest::decode(&bytes).unwrap(), manifest);
        assert_eq!(manifest.snapshot().unwrap(), manifest.snapshot().unwrap());
    }

    #[test]
    fn rejects_noncanonical_key_order_and_leb() {
        let bytes = [b'W', b'S', b'M', b'M', 1, 0, 0x81, 0x00, 0];
        assert_eq!(Manifest::decode(&bytes), Err(Error::NonCanonicalLeb));
        let mut manifest = Manifest::new();
        assert_eq!(manifest.insert("Module.Kind".into(), Value::Bool(true)), Err(Error::InvalidKey));
    }
}