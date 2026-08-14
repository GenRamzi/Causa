use anyhow::{Context, Result};
use blake3::Hasher;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use thiserror::Error;

pub const FORMAT_VERSION: &str = "0.1";

#[derive(Debug, Error)]
pub enum TapeError {
    #[error("unsupported tape format version: {0}")]
    UnsupportedVersion(String),
    #[error("tape integrity check failed: expected {expected}, calculated {actual}")]
    Integrity { expected: String, actual: String },
    #[error("signature verification failed")]
    InvalidSignature,
    #[error("invalid assertion: {0}")]
    InvalidAssertion(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Label {
    pub namespace: String,
    pub value: String,
}

impl Label {
    pub fn new(namespace: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            value: value.into(),
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        let (namespace, label) = value
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("label must have namespace:value form"))?;
        if namespace.is_empty() || label.is_empty() {
            anyhow::bail!("label namespace and value cannot be empty");
        }
        Ok(Self::new(namespace, label))
    }

    pub fn as_string(&self) -> String {
        format!("{}:{}", self.namespace, self.value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    UserMessage,
    ModelRequest,
    ModelResponse,
    ToolCall,
    ToolResult,
    FileRead,
    FileWrite,
    HttpRequest,
    StateWrite,
    GuardDecision,
    Note,
    ProcessExit,
}

impl EventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserMessage => "user.message",
            Self::ModelRequest => "model.request",
            Self::ModelResponse => "model.response",
            Self::ToolCall => "tool.call",
            Self::ToolResult => "tool.result",
            Self::FileRead => "fs.read",
            Self::FileWrite => "fs.write",
            Self::HttpRequest => "http.request",
            Self::StateWrite => "state.write",
            Self::GuardDecision => "guard.decision",
            Self::Note => "note",
            Self::ProcessExit => "process.exit",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub step: u64,
    pub kind: EventKind,
    pub name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    #[serde(default)]
    pub labels: BTreeSet<Label>,
    #[serde(default)]
    pub parents: Vec<String>,
    pub hash: String,
}

impl Event {
    pub fn new(
        step: u64,
        kind: EventKind,
        name: impl Into<String>,
        input: serde_json::Value,
        output: serde_json::Value,
        labels: impl IntoIterator<Item = Label>,
        parents: Vec<String>,
    ) -> Self {
        let mut event = Self {
            step,
            kind,
            name: name.into(),
            input,
            output,
            labels: labels.into_iter().collect(),
            parents,
            hash: String::new(),
        };
        event.hash = event.calculate_hash();
        event
    }

    fn canonical_without_hash(&self) -> Vec<u8> {
        let value = serde_json::json!({
            "step": self.step,
            "kind": self.kind,
            "name": self.name,
            "input": self.input,
            "output": self.output,
            "labels": self.labels,
            "parents": self.parents,
        });
        serde_json::to_vec(&value).expect("event serialization cannot fail")
    }

    pub fn calculate_hash(&self) -> String {
        hex::encode(blake3::hash(&self.canonical_without_hash()).as_bytes())
    }

    pub fn inherited_labels(events: &[Event]) -> BTreeSet<Label> {
        events
            .iter()
            .flat_map(|event| event.labels.iter().cloned())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TapeMetadata {
    pub run_id: String,
    pub created_at: String,
    pub command: Option<String>,
    pub platform: String,
    pub mode: String,
    pub content_policy: String,
    #[serde(default)]
    pub source_run_id: Option<String>,
    #[serde(default)]
    pub fork_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TapeSignature {
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tape {
    pub format: String,
    pub metadata: TapeMetadata,
    pub events: Vec<Event>,
    pub merkle_root: String,
    pub signature: Option<TapeSignature>,
}

impl Tape {
    pub fn new(metadata: TapeMetadata, events: Vec<Event>) -> Self {
        let merkle_root = merkle_root(&events);
        Self {
            format: FORMAT_VERSION.to_string(),
            metadata,
            events,
            merkle_root,
            signature: None,
        }
    }

    pub fn verify_integrity(&self) -> Result<()> {
        if self.format != FORMAT_VERSION {
            return Err(TapeError::UnsupportedVersion(self.format.clone()).into());
        }
        for event in &self.events {
            let actual = event.calculate_hash();
            if actual != event.hash {
                return Err(TapeError::Integrity {
                    expected: event.hash.clone(),
                    actual,
                }
                .into());
            }
        }
        let actual_root = merkle_root(&self.events);
        if actual_root != self.merkle_root {
            return Err(TapeError::Integrity {
                expected: self.merkle_root.clone(),
                actual: actual_root,
            }
            .into());
        }
        if let Some(signature) = &self.signature {
            self.verify_signature(signature)?;
        }
        Ok(())
    }

    pub fn sign(&mut self, signing_key: &SigningKey) {
        let signature = signing_key.sign(self.signing_bytes().as_slice());
        self.signature = Some(TapeSignature {
            public_key: hex::encode(signing_key.verifying_key().to_bytes()),
            signature: hex::encode(signature.to_bytes()),
        });
    }

    pub fn generate_signing_key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    fn signing_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "format": self.format,
            "metadata": self.metadata,
            "events": self.events,
            "merkle_root": self.merkle_root,
        }))
        .expect("tape signing serialization cannot fail")
    }

    fn verify_signature(&self, signature: &TapeSignature) -> Result<()> {
        let public_bytes: [u8; 32] = hex::decode(&signature.public_key)
            .map_err(|_| TapeError::InvalidSignature)?
            .try_into()
            .map_err(|_| TapeError::InvalidSignature)?;
        let signature_bytes: [u8; 64] = hex::decode(&signature.signature)
            .map_err(|_| TapeError::InvalidSignature)?
            .try_into()
            .map_err(|_| TapeError::InvalidSignature)?;
        let key =
            VerifyingKey::from_bytes(&public_bytes).map_err(|_| TapeError::InvalidSignature)?;
        key.verify(
            self.signing_bytes().as_slice(),
            &Signature::from_bytes(&signature_bytes),
        )
        .map_err(|_| TapeError::InvalidSignature.into())
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        self.verify_integrity()?;
        let path = path.as_ref();
        let tmp = path.with_extension("causa.tmp");
        let data = serde_json::to_vec_pretty(self).context("serialize tape")?;
        fs::write(&tmp, data).context("write temporary tape")?;
        fs::rename(&tmp, path).context("atomically publish tape")?;
        Ok(())
    }

    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let data = fs::read(path.as_ref()).context("read tape")?;
        let tape: Tape = serde_json::from_slice(&data).context("parse .causa tape")?;
        tape.verify_integrity()?;
        Ok(tape)
    }

    pub fn append_event(
        &mut self,
        kind: EventKind,
        name: impl Into<String>,
        input: serde_json::Value,
        output: serde_json::Value,
        labels: impl IntoIterator<Item = Label>,
    ) -> String {
        let step = self.events.len() as u64 + 1;
        let parents = self
            .events
            .last()
            .map(|event| vec![event.hash.clone()])
            .unwrap_or_default();
        let event = Event::new(step, kind, name, input, output, labels, parents);
        let hash = event.hash.clone();
        self.events.push(event);
        self.merkle_root = merkle_root(&self.events);
        self.signature = None;
        hash
    }

    pub fn fork_at(&self, step: u64, note: impl Into<String>) -> Result<Self> {
        if step == 0 || step > self.events.len() as u64 {
            anyhow::bail!("fork step is outside tape");
        }
        let mut fork = self.clone();
        fork.metadata.run_id = format!("{}-fork-{}", self.metadata.run_id, step);
        fork.metadata.mode = "fork".to_string();
        fork.metadata.source_run_id = Some(self.metadata.run_id.clone());
        fork.metadata.fork_at = Some(step);
        fork.events.truncate(step as usize);
        let parent = fork
            .events
            .last()
            .map(|event| event.hash.clone())
            .unwrap_or_default();
        let next_step = step + 1;
        fork.events.push(Event::new(
            next_step,
            EventKind::Note,
            "fork",
            serde_json::json!({"source_run_id": self.metadata.run_id, "at": step}),
            serde_json::json!({"note": note.into()}),
            [],
            vec![parent],
        ));
        fork.merkle_root = merkle_root(&fork.events);
        fork.signature = None;
        Ok(fork)
    }

    pub fn override_output(&self, step: u64, output: serde_json::Value) -> Result<Self> {
        if step == 0 || step > self.events.len() as u64 {
            anyhow::bail!("override step is outside tape");
        }
        let mut derived = self.clone();
        derived.metadata.run_id = format!("{}-replay-{}", self.metadata.run_id, step);
        derived.metadata.mode = "replay-override".to_string();
        derived.metadata.source_run_id = Some(self.metadata.run_id.clone());
        derived.metadata.fork_at = Some(step);
        derived.signature = None;
        derived.events.clear();
        for source in &self.events {
            let parents = derived
                .events
                .last()
                .map(|event| vec![event.hash.clone()])
                .unwrap_or_default();
            let event = Event::new(
                source.step,
                source.kind.clone(),
                source.name.clone(),
                source.input.clone(),
                if source.step == step {
                    output.clone()
                } else {
                    source.output.clone()
                },
                source.labels.clone(),
                parents,
            );
            derived.events.push(event);
        }
        derived.merkle_root = merkle_root(&derived.events);
        Ok(derived)
    }

    pub fn event_by_step(&self, step: u64) -> Option<&Event> {
        self.events.iter().find(|event| event.step == step)
    }

    pub fn labels_for_step(&self, step: u64) -> BTreeSet<Label> {
        let mut labels = BTreeSet::new();
        if let Some(event) = self.event_by_step(step) {
            labels.extend(event.labels.clone());
            for parent in &event.parents {
                if let Some(parent_event) = self
                    .events
                    .iter()
                    .find(|candidate| &candidate.hash == parent)
                {
                    labels.extend(parent_event.labels.clone());
                }
            }
        }
        labels
    }
}

pub fn merkle_root(events: &[Event]) -> String {
    if events.is_empty() {
        return hex::encode(blake3::hash(b"causa:empty").as_bytes());
    }
    let mut level: Vec<[u8; 32]> = events
        .iter()
        .map(|event| *blake3::hash(event.hash.as_bytes()).as_bytes())
        .collect();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let mut hasher = Hasher::new();
            hasher.update(&pair[0]);
            hasher.update(pair.get(1).unwrap_or(&pair[0]));
            next.push(*hasher.finalize().as_bytes());
        }
        level = next;
    }
    hex::encode(level[0])
}

pub struct TapeBuilder {
    metadata: TapeMetadata,
    events: Vec<Event>,
}

impl TapeBuilder {
    pub fn new(metadata: TapeMetadata) -> Self {
        Self {
            metadata,
            events: Vec::new(),
        }
    }

    pub fn push(
        &mut self,
        kind: EventKind,
        name: impl Into<String>,
        input: serde_json::Value,
        output: serde_json::Value,
        labels: impl IntoIterator<Item = Label>,
    ) -> String {
        let step = self.events.len() as u64 + 1;
        let parents = self
            .events
            .last()
            .map(|event| vec![event.hash.clone()])
            .unwrap_or_default();
        let event = Event::new(step, kind, name, input, output, labels, parents);
        let hash = event.hash.clone();
        self.events.push(event);
        hash
    }

    pub fn finish(self) -> Tape {
        Tape::new(self.metadata, self.events)
    }
}

pub fn new_run_metadata(command: Option<String>, mode: impl Into<String>) -> TapeMetadata {
    let run_id = format!(
        "run-{}",
        hex::encode(
            &blake3::hash(format!("{:?}", std::time::SystemTime::now()).as_bytes()).as_bytes()[..6]
        )
    );
    TapeMetadata {
        run_id,
        created_at: chrono_like_now(),
        command,
        platform: std::env::consts::OS.to_string(),
        mode: mode.into(),
        content_policy: "recorded-content".to_string(),
        source_run_id: None,
        fork_at: None,
    }
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("unix:{}", seconds)
}

pub fn parse_simple_assertion(assertion: &str, tape: &Tape) -> Result<bool> {
    let assertion = assertion.trim();
    let (left, right) = assertion
        .split_once("==")
        .ok_or_else(|| TapeError::InvalidAssertion(assertion.to_string()))?;
    let left = left.trim();
    let right = right.trim().trim_matches('"').trim_matches('\'');
    if left == "final.status" {
        let final_event = tape
            .events
            .last()
            .ok_or_else(|| anyhow::anyhow!("tape has no events"))?;
        return Ok(final_event
            .output
            .get("status")
            .and_then(|v| v.as_str())
            .map(|v| v == right)
            .unwrap_or(false));
    }
    if let Some(key) = left.strip_prefix("final.output.") {
        let final_event = tape
            .events
            .last()
            .ok_or_else(|| anyhow::anyhow!("tape has no events"))?;
        return Ok(final_event
            .output
            .get(key)
            .map(|v| v.to_string().trim_matches('"') == right)
            .unwrap_or(false));
    }
    Err(TapeError::InvalidAssertion(assertion.to_string()).into())
}

pub fn redact_value(
    value: &serde_json::Value,
    sensitive_keys: &BTreeSet<String>,
) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let redacted = map
                .iter()
                .map(|(key, value)| {
                    if sensitive_keys.contains(key) {
                        (
                            key.clone(),
                            serde_json::Value::String("[REDACTED]".to_string()),
                        )
                    } else {
                        (key.clone(), redact_value(value, sensitive_keys))
                    }
                })
                .collect();
            serde_json::Value::Object(redacted)
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|v| redact_value(v, sensitive_keys))
                .collect(),
        ),
        _ => value.clone(),
    }
}

pub fn labels_to_json(labels: &BTreeSet<Label>) -> serde_json::Value {
    serde_json::Value::Array(
        labels
            .iter()
            .map(|label| serde_json::Value::String(label.as_string()))
            .collect(),
    )
}

pub fn tape_summary(tape: &Tape) -> BTreeMap<String, serde_json::Value> {
    let mut result = BTreeMap::new();
    result.insert("run_id".into(), serde_json::json!(tape.metadata.run_id));
    result.insert("steps".into(), serde_json::json!(tape.events.len()));
    result.insert("merkle_root".into(), serde_json::json!(tape.merkle_root));
    result.insert("signed".into(), serde_json::json!(tape.signature.is_some()));
    result.insert(
        "kinds".into(),
        serde_json::json!(tape.events.iter().fold(
            BTreeMap::<String, usize>::new(),
            |mut map, event| {
                *map.entry(event.kind.as_str().to_string()).or_default() += 1;
                map
            }
        )),
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> TapeMetadata {
        new_run_metadata(Some("test".into()), "test")
    }

    #[test]
    fn content_hash_is_stable() {
        let event = Event::new(
            1,
            EventKind::Note,
            "hello",
            serde_json::json!({"a": 1}),
            serde_json::json!({"b": 2}),
            [Label::new("user", "trusted")],
            vec![],
        );
        assert_eq!(event.hash, event.calculate_hash());
    }

    #[test]
    fn tape_roundtrip_and_integrity() {
        let mut builder = TapeBuilder::new(metadata());
        builder.push(
            EventKind::UserMessage,
            "input",
            serde_json::json!({"text":"hi"}),
            serde_json::json!({"accepted":true}),
            [Label::new("user", "trusted")],
        );
        let tape = builder.finish();
        assert!(tape.verify_integrity().is_ok());
        let encoded = serde_json::to_vec(&tape).unwrap();
        let decoded: Tape = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.merkle_root, tape.merkle_root);
    }

    #[test]
    fn signature_verifies() {
        let mut builder = TapeBuilder::new(metadata());
        builder.push(
            EventKind::Note,
            "signed",
            serde_json::json!({}),
            serde_json::json!({"status":"ok"}),
            [],
        );
        let mut tape = builder.finish();
        let key = Tape::generate_signing_key();
        tape.sign(&key);
        assert!(tape.verify_integrity().is_ok());
    }

    #[test]
    fn fork_preserves_source_lineage() {
        let mut builder = TapeBuilder::new(metadata());
        builder.push(
            EventKind::UserMessage,
            "input",
            serde_json::json!({"text":"hi"}),
            serde_json::json!({"accepted":true}),
            [Label::new("user", "trusted")],
        );
        builder.push(
            EventKind::ToolResult,
            "search",
            serde_json::json!({}),
            serde_json::json!({"value":1}),
            [Label::new("web", "untrusted")],
        );
        let source = builder.finish();
        let fork = source.fork_at(1, "alternate result").unwrap();
        assert_eq!(fork.events[0].hash, source.events[0].hash);
        assert_eq!(fork.metadata.source_run_id, Some(source.metadata.run_id));
        assert_eq!(fork.metadata.fork_at, Some(1));
        assert!(fork.verify_integrity().is_ok());
    }

    #[test]
    fn replay_override_changes_only_selected_output() {
        let mut builder = TapeBuilder::new(metadata());
        builder.push(
            EventKind::ToolResult,
            "search",
            serde_json::json!({}),
            serde_json::json!({"value":1}),
            [],
        );
        builder.push(
            EventKind::ProcessExit,
            "final",
            serde_json::json!({}),
            serde_json::json!({"status":"ok"}),
            [],
        );
        let source = builder.finish();
        let derived = source
            .override_output(1, serde_json::json!({"value":2}))
            .unwrap();
        assert_eq!(derived.events[0].output, serde_json::json!({"value":2}));
        assert_eq!(derived.events[1].output, source.events[1].output);
        assert!(derived.verify_integrity().is_ok());
    }

    #[test]
    fn redaction_replaces_nested_sensitive_keys() {
        let keys = ["token".to_string()].into_iter().collect();
        let result = redact_value(&serde_json::json!({"nested":{"token":"secret"}}), &keys);
        assert_eq!(result["nested"]["token"], "[REDACTED]");
    }

    #[test]
    fn assertion_is_safe_and_limited() {
        let mut builder = TapeBuilder::new(metadata());
        builder.push(
            EventKind::ProcessExit,
            "exit",
            serde_json::json!({}),
            serde_json::json!({"status":"ok"}),
            [],
        );
        let tape = builder.finish();
        assert!(parse_simple_assertion("final.status == \"ok\"", &tape).unwrap());
        assert!(parse_simple_assertion("system(", &tape).is_err());
    }
}
