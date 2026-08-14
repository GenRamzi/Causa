use anyhow::{Context, Result};
use causa_core::{
    new_run_metadata, parse_simple_assertion, tape_summary, EventKind, Label, Tape, TapeBuilder,
};
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "causa", version, about = "The black box for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Demo(DemoArgs),
    Record(RecordArgs),
    Replay(ReplayArgs),
    View(ViewArgs),
    Verify(FileArgs),
    Fork(ForkArgs),
    Diff(DiffArgs),
    Bisect(BisectArgs),
    Guard(GuardArgs),
    Test(TestArgs),
    Up(UpArgs),
}

#[derive(Args, Debug)]
struct DemoArgs {
    #[arg(long, default_value = "fixtures")]
    out_dir: PathBuf,
}

#[derive(Args, Debug)]
struct RecordArgs {
    #[arg(short, long, default_value = "run.causa")]
    output: PathBuf,
    #[arg(last = true, required = true)]
    command: Vec<String>,
}

#[derive(Args, Debug)]
struct ReplayArgs {
    file: PathBuf,
    #[arg(long)]
    from: Option<u64>,
    #[arg(long)]
    set: Option<String>,
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct ViewArgs {
    file: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct FileArgs {
    file: PathBuf,
}

#[derive(Args, Debug)]
struct ForkArgs {
    file: PathBuf,
    #[arg(long)]
    at: u64,
    #[arg(short, long, default_value = "fork.causa")]
    output: PathBuf,
    #[arg(long)]
    note: Option<String>,
}

#[derive(Args, Debug)]
struct DiffArgs {
    left: PathBuf,
    right: PathBuf,
}

#[derive(Args, Debug)]
struct BisectArgs {
    bad: PathBuf,
    #[arg(long)]
    good: PathBuf,
    #[arg(long, default_value = "final.status == \"ok\"")]
    assert: String,
}

#[derive(Args, Debug)]
struct GuardArgs {
    file: PathBuf,
    #[arg(long)]
    policy: Option<PathBuf>,
    #[arg(long)]
    audit: bool,
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct TestArgs {
    #[arg(default_value = "fixtures")]
    directory: PathBuf,
}

#[derive(Args, Debug)]
struct UpArgs {
    #[arg(long, default_value_t = 7777)]
    port: u16,
    #[arg(long)]
    record: Option<PathBuf>,
    #[arg(long)]
    replay: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(false)
        .try_init()
        .ok();
    match Cli::parse().command {
        Commands::Demo(args) => demo(args),
        Commands::Record(args) => record(args),
        Commands::Replay(args) => replay(args),
        Commands::View(args) => view(args),
        Commands::Verify(args) => verify(args),
        Commands::Fork(args) => fork(args),
        Commands::Diff(args) => diff(args),
        Commands::Bisect(args) => bisect(args),
        Commands::Guard(args) => guard(args),
        Commands::Test(args) => tape_test(args),
        Commands::Up(args) => up(args).await,
    }
}

fn demo(args: DemoArgs) -> Result<()> {
    fs::create_dir_all(&args.out_dir).context("create demo output directory")?;
    let good_path = args.out_dir.join("demo-good.causa");
    let bad_path = args.out_dir.join("demo-fail.causa");
    let mut good = TapeBuilder::new(new_run_metadata(Some("causa demo".into()), "demo"));
    good.push(
        EventKind::UserMessage,
        "user.prompt",
        json!({"text":"Find the price"}),
        json!({"accepted":true}),
        [Label::new("user", "trusted")],
    );
    good.push(
        EventKind::ToolCall,
        "search.results",
        json!({"query":"price"}),
        json!({"columns":["item","currency","amount"],"rows":[["widget","USD",10]]}),
        [Label::new("web", "untrusted"), Label::new("tool", "search")],
    );
    good.push(
        EventKind::ModelResponse,
        "agent.answer",
        json!({"context_step":2}),
        json!({"status":"ok","answer":"The widget costs 10 USD."}),
        [Label::new("web", "untrusted")],
    );
    good.push(
        EventKind::ProcessExit,
        "final",
        json!({}),
        json!({"status":"ok"}),
        [],
    );
    good.finish().write(&good_path)?;

    let mut bad = TapeBuilder::new(new_run_metadata(Some("causa demo".into()), "demo"));
    bad.push(
        EventKind::UserMessage,
        "user.prompt",
        json!({"text":"Find the price"}),
        json!({"accepted":true}),
        [Label::new("user", "trusted")],
    );
    bad.push(
        EventKind::ToolCall,
        "search.results",
        json!({"query":"price"}),
        json!({"columns":["item","amount","currency"],"rows":[["widget",10,"EUR"]]}),
        [Label::new("web", "untrusted"), Label::new("tool", "search")],
    );
    bad.push(
        EventKind::ModelResponse,
        "agent.answer",
        json!({"context_step":2}),
        json!({"status":"error","answer":"Currency column changed."}),
        [Label::new("web", "untrusted")],
    );
    bad.push(
        EventKind::ProcessExit,
        "final",
        json!({}),
        json!({"status":"error"}),
        [],
    );
    bad.finish().write(&bad_path)?;

    println!("Causa demo · recorded deterministic failure");
    println!("  good tape  {}", good_path.display());
    println!("  bad tape   {}", bad_path.display());
    println!("\nReplay the failure:");
    println!("  causa replay {}", bad_path.display());
    println!("Find the root cause:");
    println!(
        "  causa bisect {} --good {} --assert 'final.status == \"ok\"'",
        bad_path.display(),
        good_path.display()
    );
    Ok(())
}

fn record(args: RecordArgs) -> Result<()> {
    let program = args
        .command
        .first()
        .context("record requires a command after --")?;
    let output = Command::new(program)
        .args(&args.command[1..])
        .output()
        .with_context(|| format!("run {}", program))?;
    let mut builder = TapeBuilder::new(new_run_metadata(Some(args.command.join(" ")), "record"));
    let command = args.command.join(" ");
    builder.push(
        EventKind::Note,
        "process.start",
        json!({"command":command}),
        json!({"status":"started"}),
        [],
    );
    builder.push(
        EventKind::ProcessExit,
        "process.exit",
        json!({"command":command}),
        json!({
            "status": if output.status.success() { "ok" } else { "error" },
            "code": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }),
        [],
    );
    let tape = builder.finish();
    tape.write(&args.output)?;
    println!(
        "recorded {} steps → {}",
        tape.events.len(),
        args.output.display()
    );
    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }
    Ok(())
}

fn replay(args: ReplayArgs) -> Result<()> {
    let tape = Tape::read(&args.file)?;
    let derived = if let Some(spec) = &args.set {
        let (step, fixture) = parse_override_spec(spec)?;
        let value: serde_json::Value = serde_json::from_slice(
            &fs::read(&fixture)
                .with_context(|| format!("read override fixture {}", fixture.display()))?,
        )
        .with_context(|| format!("parse override fixture {}", fixture.display()))?;
        tape.override_output(step, value)?
    } else {
        tape.clone()
    };
    if let Some(output) = &args.output {
        derived.write(output)?;
        println!("derived replay tape → {}", output.display());
    }
    let start = args.from.unwrap_or(1);
    println!(
        "causa replay · {} · {} steps · offline",
        derived.metadata.run_id,
        derived.events.len()
    );
    if let Some(set) = args.set {
        println!("override: {}", set);
    }
    for event in derived.events.iter().filter(|event| event.step >= start) {
        println!(
            "  step {:>3}  {:<16} {:<24} {}",
            event.step,
            event.kind.as_str(),
            event.name,
            &event.hash[..12]
        );
    }
    println!("replay complete · final root {}", derived.merkle_root);
    Ok(())
}

fn parse_override_spec(spec: &str) -> Result<(u64, PathBuf)> {
    let (step_text, path_text) = spec
        .split_once('@')
        .context("override must use step:N=@fixture.json")?;
    let step_text = step_text.strip_prefix("step:").unwrap_or(step_text);
    let step = step_text
        .parse::<u64>()
        .context("override step must be an integer")?;
    anyhow::ensure!(step > 0, "override step must be greater than zero");
    anyhow::ensure!(
        !path_text.is_empty(),
        "override fixture path cannot be empty"
    );
    Ok((step, PathBuf::from(path_text)))
}

fn view(args: ViewArgs) -> Result<()> {
    let tape = Tape::read(&args.file)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&tape)?);
        return Ok(());
    }
    println!("Causa tape {}", tape.metadata.run_id);
    println!(
        "format={} platform={} mode={} root={}",
        tape.format, tape.metadata.platform, tape.metadata.mode, tape.merkle_root
    );
    for event in &tape.events {
        let labels = event
            .labels
            .iter()
            .map(Label::as_string)
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{:>3}  {:<16} {:<24} labels=[{}]",
            event.step,
            event.kind.as_str(),
            event.name,
            labels
        );
    }
    Ok(())
}

fn verify(args: FileArgs) -> Result<()> {
    let tape = Tape::read(&args.file)?;
    println!(
        "verified {} · {} steps · merkle {}",
        args.file.display(),
        tape.events.len(),
        tape.merkle_root
    );
    if tape.signature.is_some() {
        println!("signature: valid");
    } else {
        println!("signature: absent (integrity verified)");
    }
    Ok(())
}

fn fork(args: ForkArgs) -> Result<()> {
    let tape = Tape::read(&args.file)?;
    let forked = tape.fork_at(
        args.at,
        args.note.unwrap_or_else(|| "alternate timeline".into()),
    )?;
    forked.write(&args.output)?;
    println!("forked at step {} → {}", args.at, args.output.display());
    println!(
        "source run {} · derived run {}",
        tape.metadata.run_id, forked.metadata.run_id
    );
    Ok(())
}

fn diff(args: DiffArgs) -> Result<()> {
    let left = Tape::read(&args.left)?;
    let right = Tape::read(&args.right)?;
    let max = left.events.len().max(right.events.len());
    let mut differences = 0;
    for index in 0..max {
        let a = left.events.get(index);
        let b = right.events.get(index);
        if a.map(|e| &e.hash) != b.map(|e| &e.hash) {
            differences += 1;
            println!(
                "step {} differs: {} → {}",
                index + 1,
                a.map(|e| e.name.as_str()).unwrap_or("<missing>"),
                b.map(|e| e.name.as_str()).unwrap_or("<missing>")
            );
            if let (Some(a), Some(b)) = (a, b) {
                println!("  changed fields: {}", changed_fields(a, b).join(", "));
            }
        }
    }
    println!(
        "semantic diff · {} differing nodes · first divergence {}",
        differences,
        if differences == 0 {
            "none".into()
        } else {
            (left
                .events
                .iter()
                .zip(right.events.iter())
                .position(|(a, b)| a.hash != b.hash)
                .unwrap_or(left.events.len())
                + 1)
            .to_string()
        }
    );
    Ok(())
}

fn changed_fields(left: &causa_core::Event, right: &causa_core::Event) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if left.kind != right.kind {
        fields.push("kind");
    }
    if left.name != right.name {
        fields.push("name");
    }
    if left.input != right.input {
        fields.push("input");
    }
    if left.output != right.output {
        fields.push("output");
    }
    if left.labels != right.labels {
        fields.push("labels");
    }
    if left.parents != right.parents {
        fields.push("parents");
    }
    fields
}

fn bisect(args: BisectArgs) -> Result<()> {
    let bad = Tape::read(&args.bad)?;
    let good = Tape::read(&args.good)?;
    let good_pass = parse_simple_assertion(&args.assert, &good)?;
    let bad_pass = parse_simple_assertion(&args.assert, &bad)?;
    anyhow::ensure!(
        good_pass && !bad_pass,
        "assertion must pass on good tape and fail on bad tape"
    );
    let first = bad
        .events
        .iter()
        .zip(good.events.iter())
        .position(|(a, b)| a.hash != b.hash)
        .map(|index| index + 1)
        .unwrap_or_else(|| {
            if bad.events.len() != good.events.len() {
                bad.events.len().min(good.events.len()) + 1
            } else {
                0
            }
        });
    if first == 0 {
        println!("no divergent node found");
        return Ok(());
    }
    let event = &bad.events[first - 1];
    let descendants = bad
        .events
        .iter()
        .filter(|candidate| candidate.step >= event.step)
        .count();
    println!("causa bisect · causal delta-debugging");
    println!("  probes              1 (fixture-guided)");
    println!(
        "  ROOT CAUSE          step {} — {}:{}",
        event.step,
        event.kind.as_str(),
        event.name
    );
    println!("  first-divergent-node {}", event.hash);
    println!(
        "  blast radius        steps {} → {}",
        event.step,
        bad.events.last().map(|e| e.step).unwrap_or(event.step)
    );
    println!("  affected nodes      {}", descendants);
    println!(
        "  minimal repro       causa replay {} --from {}",
        args.bad.display(),
        event.step
    );
    Ok(())
}

fn tape_test(args: TestArgs) -> Result<()> {
    let mut files = Vec::new();
    for entry in fs::read_dir(&args.directory)
        .with_context(|| format!("read tape directory {}", args.directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("causa") {
            files.push(path);
        }
    }
    files.sort();
    anyhow::ensure!(
        !files.is_empty(),
        "no .causa tapes found in {}",
        args.directory.display()
    );
    let mut failures = 0usize;
    for path in &files {
        match Tape::read(path) {
            Ok(tape) => println!("PASS {:<40} {} steps", path.display(), tape.events.len()),
            Err(error) => {
                failures += 1;
                println!("FAIL {:<40} {}", path.display(), error);
            }
        }
    }
    println!(
        "tape test · {} files · {} failure(s)",
        files.len(),
        failures
    );
    anyhow::ensure!(failures == 0, "tape regression suite failed");
    Ok(())
}

#[derive(Debug, Clone)]
enum GuardCondition {
    WebTaint,
    PiiTaint,
    WorkspaceWrite,
}

#[derive(Debug, Clone)]
struct GuardRule {
    effect: String,
    target: String,
    condition: GuardCondition,
}

fn parse_guard_policy(policy_text: &str) -> Result<Vec<GuardRule>> {
    let mut rules = Vec::new();
    for (index, raw) in policy_text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line == "policies:" {
            continue;
        }
        let (effect_target, condition_text) = line.split_once(" when ").with_context(|| {
            format!(
                "invalid policy line {}: expected '<effect>: <target> when <condition>'",
                index + 1
            )
        })?;
        let (effect, target) = effect_target
            .split_once(':')
            .with_context(|| format!("invalid policy line {}: missing effect", index + 1))?;
        let condition = if condition_text.contains("tainted_by(source.web)") {
            GuardCondition::WebTaint
        } else if condition_text.contains("label(pii)") {
            GuardCondition::PiiTaint
        } else if condition_text.contains("path under ./workspace") {
            GuardCondition::WorkspaceWrite
        } else {
            anyhow::bail!("unsupported guard condition on line {}", index + 1);
        };
        rules.push(GuardRule {
            effect: effect.trim().to_string(),
            target: target.trim().to_string(),
            condition,
        });
    }
    Ok(rules)
}

fn guard(args: GuardArgs) -> Result<()> {
    let tape = Tape::read(&args.file)?;
    let policy_text = args.policy.as_ref().map(fs::read_to_string).transpose()?.unwrap_or_else(|| "deny: tool.email.send when tainted_by(source.web)\ndeny: tool.http.post when context has label(pii)".into());
    let rules = parse_guard_policy(&policy_text)?;
    let mut blocked = Vec::new();
    let mut annotated = tape.clone();
    annotated.metadata.mode = "guarded".to_string();
    annotated.metadata.run_id = format!("{}-guard", tape.metadata.run_id);
    for event in &tape.events {
        let labels = tape.labels_for_step(event.step);
        for rule in &rules {
            let target_match = event.name == rule.target || event.name.ends_with(&rule.target);
            let condition_match = match rule.condition {
                GuardCondition::WebTaint => labels.contains(&Label::new("web", "untrusted")),
                GuardCondition::PiiTaint => labels.contains(&Label::new("db", "pii")),
                GuardCondition::WorkspaceWrite => event
                    .input
                    .get("path")
                    .and_then(|value| value.as_str())
                    .map(|path| path.starts_with("./workspace/"))
                    .unwrap_or(false),
            };
            if target_match && condition_match {
                let denied = rule.effect == "deny";
                let decision = if denied && args.audit {
                    "audit"
                } else if denied {
                    "deny"
                } else {
                    "allow"
                };
                println!(
                    "{} step {} {} by {}",
                    decision.to_uppercase(),
                    event.step,
                    event.name,
                    rule.target
                );
                annotated.append_event(
                    EventKind::GuardDecision,
                    "guard.decision",
                    json!({"event_step":event.step,"rule":rule.target}),
                    json!({"decision":decision}),
                    labels.clone(),
                );
                if denied {
                    blocked.push(event.step);
                }
            }
        }
    }
    if let Some(output) = &args.output {
        annotated.write(output)?;
        println!("guard decision tape → {}", output.display());
    }
    if blocked.is_empty() {
        println!("guard passed · no policy violations");
        Ok(())
    } else if args.audit {
        println!("audit complete · {} potential violation(s)", blocked.len());
        Ok(())
    } else {
        anyhow::bail!("guard blocked {} event(s)", blocked.len())
    }
}

#[derive(Clone)]
struct ProxyConfig {
    record: Option<PathBuf>,
    replay: Option<PathBuf>,
}

async fn up(args: UpArgs) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    let listener = TcpListener::bind(("127.0.0.1", args.port))
        .await
        .context("bind local proxy")?;
    let config = ProxyConfig {
        record: args.record.clone(),
        replay: args.replay.clone(),
    };
    println!("causa proxy listening on http://127.0.0.1:{}", args.port);
    if let Some(path) = &config.record {
        println!("recording requests to {}", path.display());
    }
    if let Some(path) = &config.replay {
        println!("replaying responses from {}", path.display());
    }
    loop {
        let (mut socket, _) = listener.accept().await?;
        let config = config.clone();
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 64 * 1024];
            let size = match socket.read(&mut buffer).await {
                Ok(size) if size > 0 => size,
                _ => return,
            };
            buffer.truncate(size);
            let request = String::from_utf8_lossy(&buffer).to_string();
            let response = match handle_proxy_request(&request, &config) {
                Ok(response) => response,
                Err(error) => http_json(
                    "400 Bad Request",
                    serde_json::json!({"error": error.to_string()}),
                ),
            };
            let _ = socket.write_all(response.as_bytes()).await;
        });
    }
}

fn handle_proxy_request(request: &str, config: &ProxyConfig) -> Result<String> {
    let mut lines = request.split("\r\n");
    let first = lines.next().context("missing HTTP request line")?;
    let path = first.split_whitespace().nth(1).unwrap_or("/");
    if path == "/health" {
        return Ok(http_json(
            "200 OK",
            json!({"status":"ok","service":"causa"}),
        ));
    }
    if path == "/v1/models" {
        return Ok(http_json(
            "200 OK",
            json!({"object":"list","data":[{"id":"causa-replay","object":"model"}]}),
        ));
    }
    if path != "/v1/chat/completions" {
        return Ok(http_json("404 Not Found", json!({"error":"not found"})));
    }
    let body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("");
    let request_json: serde_json::Value =
        serde_json::from_str(body).context("invalid JSON request body")?;
    let stream = request_json
        .get("stream")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let content = if let Some(path) = &config.replay {
        replay_content(path)?
    } else {
        "Causa proxy is ready. This response was captured locally and can be replayed offline."
            .to_string()
    };
    let response_json = openai_response(&content);
    if let Some(path) = &config.record {
        let mut tape = if path.exists() {
            Tape::read(path)?
        } else {
            Tape::new(
                new_run_metadata(Some("causa proxy".into()), "proxy"),
                Vec::new(),
            )
        };
        tape.append_event(
            EventKind::ModelRequest,
            "openai.chat.completions",
            request_json.clone(),
            json!({"accepted":true}),
            [],
        );
        tape.append_event(
            EventKind::ModelResponse,
            "openai.chat.completions",
            json!({"stream":stream}),
            response_json.clone(),
            [],
        );
        tape.write(path)?;
    }
    if stream {
        Ok(http_sse(&content))
    } else {
        Ok(http_json("200 OK", response_json))
    }
}

fn replay_content(path: &Path) -> Result<String> {
    let tape = Tape::read(path)?;
    let event = tape
        .events
        .iter()
        .rev()
        .find(|event| matches!(event.kind, EventKind::ModelResponse));
    let Some(event) = event else {
        anyhow::bail!("replay tape has no model.response event");
    };
    if let Some(content) = event
        .output
        .pointer("/choices/0/message/content")
        .and_then(|value| value.as_str())
    {
        return Ok(content.to_string());
    }
    if let Some(answer) = event.output.get("answer").and_then(|value| value.as_str()) {
        return Ok(answer.to_string());
    }
    Ok(event.output.to_string())
}

fn openai_response(content: &str) -> serde_json::Value {
    json!({"id":"causa-local","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}],"usage":{"prompt_tokens":0,"completion_tokens":content.split_whitespace().count(),"total_tokens":content.split_whitespace().count()}})
}

fn http_json(status: &str, body: serde_json::Value) -> String {
    let body = body.to_string();
    format!("HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", status, body.len(), body)
}

fn http_sse(content: &str) -> String {
    let first = json!({"id":"causa-local","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"role":"assistant","content":content},"finish_reason":null}]}).to_string();
    let done = json!({"id":"causa-local","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}).to_string();
    let body = format!("data: {}\n\ndata: {}\n\ndata: [DONE]\n\n", first, done);
    format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
}

#[allow(dead_code)]
fn _labels(value: &[&str]) -> BTreeSet<Label> {
    value.iter().filter_map(|v| Label::parse(v).ok()).collect()
}

#[allow(dead_code)]
fn _summary(path: &Path) -> Result<()> {
    let tape = Tape::read(path)?;
    println!("{}", serde_json::to_string_pretty(&tape_summary(&tape))?);
    Ok(())
}
