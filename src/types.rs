//! Guest-side types mirroring the WAFER host types.
//!
//! These types are designed for use inside WebAssembly and communicate with the
//! host runtime through JSON serialization over linear memory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Meta key constants (mirrors host meta.rs)
// ---------------------------------------------------------------------------

pub const META_REQ_ACTION: &str = "req.action";
pub const META_REQ_RESOURCE: &str = "req.resource";
pub const META_REQ_PARAM_PREFIX: &str = "req.param.";
pub const META_REQ_QUERY_PREFIX: &str = "req.query.";
pub const META_REQ_CLIENT_IP: &str = "req.client.ip";
pub const META_REQ_CONTENT_TYPE: &str = "req.content_type";

pub const META_AUTH_USER_ID: &str = "auth.user_id";
pub const META_AUTH_USER_EMAIL: &str = "auth.user_email";
pub const META_AUTH_USER_ROLES: &str = "auth.user_roles";

pub const META_RESP_STATUS: &str = "resp.status";
pub const META_RESP_CONTENT_TYPE: &str = "resp.content_type";
pub const META_RESP_HEADER_PREFIX: &str = "resp.header.";
pub const META_RESP_COOKIE_PREFIX: &str = "resp.set_cookie.";

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

/// A message flowing through the WAFER pipeline. Contains a kind identifier,
/// payload data, and key-value metadata.
#[derive(Debug, Clone)]
pub struct Message {
    pub kind: String,
    pub data: Vec<u8>,
    pub meta: HashMap<String, String>,
}

impl Message {
    /// Create a new message with the given kind and data.
    pub fn new(kind: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self {
            kind: kind.into(),
            data: data.into(),
            meta: HashMap::new(),
        }
    }

    /// Deserialize the data payload as JSON into the given type.
    pub fn unmarshal<T: serde::de::DeserializeOwned>(&self) -> std::result::Result<T, serde_json::Error> {
        serde_json::from_slice(&self.data)
    }

    /// Alias for [`unmarshal`](Self::unmarshal).
    pub fn decode<T: serde::de::DeserializeOwned>(&self) -> std::result::Result<T, serde_json::Error> {
        self.unmarshal()
    }

    /// Get a metadata value by key, returning an empty string if absent.
    pub fn get_meta(&self, key: &str) -> &str {
        self.meta.get(key).map(|s| s.as_str()).unwrap_or("")
    }

    /// Set a metadata key-value pair.
    pub fn set_meta(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.meta.insert(key.into(), value.into());
    }

    /// Serialize `v` as JSON and set it as the data payload.
    pub fn set_data<T: Serialize>(&mut self, v: &T) -> std::result::Result<(), serde_json::Error> {
        self.data = serde_json::to_vec(v)?;
        Ok(())
    }

    /// Return a [`Result_`] that passes this message to the next block.
    pub fn cont(self) -> Result_ {
        Result_ {
            action: Action::Continue,
            response: None,
            error: None,
            message: Some(self),
        }
    }

    /// Return a [`Result_`] that short-circuits the chain with a response.
    pub fn respond(self, r: Response) -> Result_ {
        Result_ {
            action: Action::Respond,
            response: Some(r),
            error: None,
            message: Some(self),
        }
    }

    /// Return a [`Result_`] that silently drops this message.
    pub fn drop_msg(self) -> Result_ {
        Result_ {
            action: Action::Drop,
            response: None,
            error: None,
            message: Some(self),
        }
    }

    /// Return a [`Result_`] that short-circuits the chain with an error.
    pub fn err(self, e: WaferError) -> Result_ {
        Result_ {
            action: Action::Error,
            response: None,
            error: Some(e),
            message: Some(self),
        }
    }

    /// Return a path variable extracted by the router.
    pub fn var(&self, name: &str) -> &str {
        let key = format!("{}{}", META_REQ_PARAM_PREFIX, name);
        self.meta.get(&key).map(|s| s.as_str()).unwrap_or("")
    }

    /// Return a query parameter value.
    pub fn query(&self, name: &str) -> &str {
        let key = format!("{}{}", META_REQ_QUERY_PREFIX, name);
        self.meta.get(&key).map(|s| s.as_str()).unwrap_or("")
    }

    /// Return a request header value.
    pub fn header(&self, name: &str) -> &str {
        let key = format!("http.header.{}", name);
        self.meta.get(&key).map(|s| s.as_str()).unwrap_or("")
    }

    /// Return the semantic request action (retrieve, create, update, delete, execute).
    pub fn action(&self) -> &str {
        self.get_meta(META_REQ_ACTION)
    }

    /// Return the request resource path.
    pub fn path(&self) -> &str {
        self.get_meta(META_REQ_RESOURCE)
    }

    /// Return the request content type.
    pub fn content_type(&self) -> &str {
        self.get_meta(META_REQ_CONTENT_TYPE)
    }

    /// Return the authenticated user's ID.
    pub fn user_id(&self) -> &str {
        self.get_meta(META_AUTH_USER_ID)
    }

    /// Return the authenticated user's email.
    pub fn user_email(&self) -> &str {
        self.get_meta(META_AUTH_USER_EMAIL)
    }

    /// Return the authenticated user's roles.
    pub fn user_roles(&self) -> Vec<&str> {
        let roles = self.get_meta(META_AUTH_USER_ROLES);
        if roles.is_empty() {
            Vec::new()
        } else {
            roles.split(',').collect()
        }
    }

    /// Return true if the authenticated user has the "admin" role.
    pub fn is_admin(&self) -> bool {
        self.user_roles().contains(&"admin")
    }

    /// Return all query parameters as a map.
    pub fn query_params(&self) -> HashMap<&str, &str> {
        self.meta
            .iter()
            .filter(|(k, _)| k.starts_with(META_REQ_QUERY_PREFIX))
            .map(|(k, v)| (&k[META_REQ_QUERY_PREFIX.len()..], v.as_str()))
            .collect()
    }

    /// Return a named cookie value from the Cookie header.
    pub fn cookie(&self, name: &str) -> &str {
        let raw = self.get_meta("http.header.Cookie");
        if raw.is_empty() {
            return "";
        }
        for part in raw.split(';') {
            let part = part.trim();
            if let Some(eq) = part.find('=') {
                if &part[..eq] == name {
                    return &part[eq + 1..];
                }
            }
        }
        ""
    }

    /// Return the client's remote address.
    pub fn remote_addr(&self) -> &str {
        self.get_meta(META_REQ_CLIENT_IP)
    }

    /// Return the raw request body.
    pub fn body(&self) -> &[u8] {
        &self.data
    }

    /// Extract page, page_size, and offset from query params.
    pub fn pagination_params(&self, default_page_size: usize) -> (usize, usize, usize) {
        let page = self
            .query("page")
            .parse::<usize>()
            .ok()
            .filter(|&p| p > 0)
            .unwrap_or(1);

        let page_size = self
            .query("page_size")
            .parse::<usize>()
            .ok()
            .filter(|&ps| ps > 0 && ps <= 100)
            .unwrap_or(default_page_size);

        let offset = (page - 1) * page_size;
        (page, page_size, offset)
    }
}

// ---------------------------------------------------------------------------
// RequestAction
// ---------------------------------------------------------------------------

/// Semantic request action (transport-agnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestAction {
    Retrieve,
    Create,
    Update,
    Delete,
    Execute,
}

impl RequestAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retrieve => "retrieve",
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Execute => "execute",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "retrieve" => Some(Self::Retrieve),
            "create" => Some(Self::Create),
            "update" => Some(Self::Update),
            "delete" => Some(Self::Delete),
            "execute" => Some(Self::Execute),
            _ => None,
        }
    }
}

impl fmt::Display for RequestAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

/// Tells the runtime what to do after a block processes a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Continue,
    Respond,
    Drop,
    Error,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Respond => "respond",
            Self::Drop => "drop",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

/// Carries data back to the caller when a block short-circuits the pipeline.
#[derive(Debug, Clone)]
pub struct Response {
    pub data: Vec<u8>,
    pub meta: HashMap<String, String>,
}

impl Response {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            meta: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// WaferError
// ---------------------------------------------------------------------------

/// A structured error returned by a block.
#[derive(Debug, Clone)]
pub struct WaferError {
    pub code: String,
    pub message: String,
    pub meta: HashMap<String, String>,
}

impl WaferError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            meta: HashMap::new(),
        }
    }

    /// Return a copy with the given metadata key-value added.
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for WaferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for WaferError {}

// ---------------------------------------------------------------------------
// Result_
// ---------------------------------------------------------------------------

/// The outcome of a block processing a message.
/// Named `Result_` to avoid conflict with `std::result::Result`.
#[derive(Debug, Clone)]
pub struct Result_ {
    pub action: Action,
    pub response: Option<Response>,
    pub error: Option<WaferError>,
    pub message: Option<Message>,
}

impl Result_ {
    /// Create a continue result carrying the given message.
    pub fn continue_with(msg: Message) -> Self {
        Self {
            action: Action::Continue,
            response: None,
            error: None,
            message: Some(msg),
        }
    }

    /// Create an error result.
    pub fn error(err: WaferError) -> Self {
        Self {
            action: Action::Error,
            response: None,
            error: Some(err),
            message: None,
        }
    }
}

// ---------------------------------------------------------------------------
// BlockInfo
// ---------------------------------------------------------------------------

/// Declares a block's identity and capabilities.
#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub name: String,
    pub version: String,
    pub interface: String,
    pub summary: String,
    pub instance_mode: InstanceMode,
    pub allowed_modes: Vec<InstanceMode>,
}

// ---------------------------------------------------------------------------
// InstanceMode
// ---------------------------------------------------------------------------

/// Controls how many block instances the runtime creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstanceMode {
    PerNode,
    Singleton,
    PerChain,
    PerExecution,
}

impl InstanceMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PerNode => "per-node",
            Self::Singleton => "singleton",
            Self::PerChain => "per-chain",
            Self::PerExecution => "per-execution",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "per-node" | "" => Some(Self::PerNode),
            "singleton" => Some(Self::Singleton),
            "per-chain" => Some(Self::PerChain),
            "per-execution" => Some(Self::PerExecution),
            _ => None,
        }
    }
}

impl fmt::Display for InstanceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// LifecycleEvent
// ---------------------------------------------------------------------------

/// Identifies the kind of lifecycle event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleType {
    Init,
    Start,
    Stop,
}

impl LifecycleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Start => "start",
            Self::Stop => "stop",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "init" => Some(Self::Init),
            "start" => Some(Self::Start),
            "stop" => Some(Self::Stop),
            _ => None,
        }
    }
}

/// A lifecycle event sent to blocks during runtime transitions.
#[derive(Debug, Clone)]
pub struct LifecycleEvent {
    pub event_type: LifecycleType,
    pub data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Wire-format types (JSON serialization over the WASM boundary)
// ---------------------------------------------------------------------------

/// Base64 serde for binary data. Matches Go's json.Marshal([]byte) convention.
mod base64_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = base64_encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            return Ok(Vec::new());
        }
        base64_decode(&s).map_err(serde::de::Error::custom)
    }

    fn base64_encode(input: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::with_capacity((input.len() + 2) / 3 * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let triple = (b0 << 16) | (b1 << 8) | b2;
            output.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            output.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                output.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
            } else {
                output.push('=');
            }
            if chunk.len() > 2 {
                output.push(CHARS[(triple & 0x3F) as usize] as char);
            } else {
                output.push('=');
            }
        }
        output
    }

    fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
        let input = input.trim_end_matches('=');
        let mut output = Vec::with_capacity(input.len() * 3 / 4);
        let mut buf: u32 = 0;
        let mut bits: u32 = 0;
        for c in input.chars() {
            let val = match c {
                'A'..='Z' => (c as u32) - ('A' as u32),
                'a'..='z' => (c as u32) - ('a' as u32) + 26,
                '0'..='9' => (c as u32) - ('0' as u32) + 52,
                '+' => 62,
                '/' => 63,
                _ => return Err(format!("invalid base64 character: {}", c)),
            };
            buf = (buf << 6) | val;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push(((buf >> bits) & 0xFF) as u8);
            }
        }
        Ok(output)
    }
}

/// JSON-serializable message that crosses the WASM boundary.
/// Data is base64-encoded for compatibility with Go's json.Marshal([]byte).
#[derive(Debug, Serialize, Deserialize)]
pub struct WasmMessage {
    pub kind: String,
    #[serde(with = "base64_serde", default)]
    pub data: Vec<u8>,
    #[serde(default)]
    pub meta: Vec<[String; 2]>,
}

/// JSON-serializable result that crosses the WASM boundary.
#[derive(Debug, Serialize, Deserialize)]
pub struct WasmResult {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<WasmResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WasmError>,
}

/// JSON-serializable response in the wire format.
#[derive(Debug, Serialize, Deserialize)]
pub struct WasmResponse {
    #[serde(with = "base64_serde", default)]
    pub data: Vec<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub meta: Vec<[String; 2]>,
}

/// JSON-serializable error in the wire format.
#[derive(Debug, Serialize, Deserialize)]
pub struct WasmError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub meta: Vec<[String; 2]>,
}

/// JSON-serializable block info in the wire format.
#[derive(Debug, Serialize, Deserialize)]
pub struct WasmBlockInfo {
    pub name: String,
    pub version: String,
    pub interface: String,
    pub summary: String,
    #[serde(default)]
    pub instance_mode: String,
    #[serde(default)]
    pub allowed_modes: Vec<String>,
}

/// JSON-serializable lifecycle event in the wire format.
#[derive(Debug, Serialize, Deserialize)]
pub struct WasmLifecycleEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(with = "base64_serde", default)]
    pub data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

impl Message {
    /// Convert from the wire format into a [`Message`].
    pub fn from_wasm(wm: WasmMessage) -> Self {
        let mut meta = HashMap::new();
        for pair in wm.meta {
            meta.insert(pair[0].clone(), pair[1].clone());
        }
        Self {
            kind: wm.kind,
            data: wm.data,
            meta,
        }
    }

    /// Convert this message into the wire format.
    pub fn to_wasm(&self) -> WasmMessage {
        WasmMessage {
            kind: self.kind.clone(),
            data: self.data.clone(),
            meta: self
                .meta
                .iter()
                .map(|(k, v)| [k.clone(), v.clone()])
                .collect(),
        }
    }
}

impl Result_ {
    /// Convert this result into the wire format.
    pub fn to_wasm(&self) -> WasmResult {
        WasmResult {
            action: self.action.as_str().to_string(),
            response: self.response.as_ref().map(|r| WasmResponse {
                data: r.data.clone(),
                meta: r
                    .meta
                    .iter()
                    .map(|(k, v)| [k.clone(), v.clone()])
                    .collect(),
            }),
            error: self.error.as_ref().map(|e| WasmError {
                code: e.code.clone(),
                message: e.message.clone(),
                meta: e
                    .meta
                    .iter()
                    .map(|(k, v)| [k.clone(), v.clone()])
                    .collect(),
            }),
        }
    }

    /// Convert from the wire format into a [`Result_`].
    pub fn from_wasm(wr: WasmResult) -> Self {
        let action = match wr.action.as_str() {
            "continue" => Action::Continue,
            "respond" => Action::Respond,
            "drop" => Action::Drop,
            "error" => Action::Error,
            _ => Action::Continue,
        };

        let response = wr.response.map(|r| {
            let mut meta = HashMap::new();
            for pair in r.meta {
                meta.insert(pair[0].clone(), pair[1].clone());
            }
            Response { data: r.data, meta }
        });

        let error = wr.error.map(|e| {
            let mut meta = HashMap::new();
            for pair in e.meta {
                meta.insert(pair[0].clone(), pair[1].clone());
            }
            WaferError {
                code: e.code,
                message: e.message,
                meta,
            }
        });

        Self {
            action,
            response,
            error,
            message: None,
        }
    }
}

impl BlockInfo {
    /// Convert this info into the wire format.
    pub fn to_wasm(&self) -> WasmBlockInfo {
        WasmBlockInfo {
            name: self.name.clone(),
            version: self.version.clone(),
            interface: self.interface.clone(),
            summary: self.summary.clone(),
            instance_mode: self.instance_mode.as_str().to_string(),
            allowed_modes: self
                .allowed_modes
                .iter()
                .map(|m| m.as_str().to_string())
                .collect(),
        }
    }
}

impl LifecycleEvent {
    /// Convert from the wire format into a [`LifecycleEvent`].
    pub fn from_wasm(wle: WasmLifecycleEvent) -> Option<Self> {
        LifecycleType::parse(&wle.event_type).map(|event_type| Self {
            event_type,
            data: wle.data,
        })
    }
}
