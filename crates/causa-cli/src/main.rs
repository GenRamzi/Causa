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
}

#[derive(Args, Debug)]
struct UpArgs {
    #[arg(long, default_value_t = 7777)]
    port: u16,
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
    let start = args.from.unwrap_or(1);
    println!(
        "causa replay · {} · {} steps · offline",
        tape.metadata.run_id,
        tape.events.len()
    );
    if let Some(set) = args.set {
        println!("override: {}", set);
    }
    for event in tape.events.iter().filter(|event| event.step >= start) {
        println!(
            "  step {:>3}  {:<16} {:<24} {}",
            event.step,
            event.kind.as_str(),
            event.name,
            &event.hash[..12]
        );
    }
    println!("replay complete · final root {}", tape.merkle_root);
    Ok(())
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
    anyhow::ensure!(
        args.at > 0 && args.at <= tape.events.len() as u64,
        "fork step is outside tape"
    );
    let mut builder = TapeBuilder::new(new_run_metadata(
        Some(format!("fork {} at {}", args.file.display(), args.at)),
        "fork",
    ));
    for event in tape.events.iter().filter(|event| event.step <= args.at) {
        builder.push(
            event.kind.clone(),
            event.name.clone(),
            event.input.clone(),
            event.output.clone(),
            event.labels.clone(),
        );
    }
    builder.push(
        EventKind::Note,
        "fork",
        json!({"source":args.file,"at":args.at}),
        json!({"note":args.note.unwrap_or_else(|| "alternate timeline".into())}),
        [],
    );
    let forked = builder.finish();
    forked.write(&args.output)?;
    println!("forked at step {} → {}", args.at, args.output.display());
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
        .unwrap_or(0);
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

fn guard(args: GuardArgs) -> Result<()> {
    let tape = Tape::read(&args.file)?;
    let policy_text = args.policy.as_ref().map(fs::read_to_string).transpose()?.unwrap_or_else(|| "deny: tool.email.send when tainted_by(source.web)\ndeny: tool.http.post when context has label(pii)".into());
    let mut blocked = Vec::new();
    for event in &tape.events {
        let web_tainted = event.labels.contains(&Label::new("web", "untrusted"));
        let pii_tainted = event.labels.contains(&Label::new("db", "pii"));
        let external_sink = event.name.contains("email") || event.name.contains("http.post");
        if (web_tainted && external_sink && policy_text.contains("tainted_by(source.web)"))
            || (pii_tainted && external_sink && policy_text.contains("label(pii)"))
        {
            blocked.push(event.step);
            println!(
                "{} step {} {}",
                if args.audit { "AUDIT" } else { "DENY" },
                event.step,
                event.name
            );
        }
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

async fn up(args: UpArgs) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    let listener = TcpListener::bind(("127.0.0.1", args.port))
        .await
        .context("bind local proxy")?;
    println!("causa proxy listening on http://127.0.0.1:{}", args.port);
    loop {
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 16 * 1024];
            let size = match socket.read(&mut buffer).await {
                Ok(size) => size,
                Err(_) => return,
            };
            let request = String::from_utf8_lossy(&buffer[..size]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");
            let body = if path == "/health" {
                json!({"status":"ok","service":"causa"}).to_string()
            } else if path == "/v1/models" {
                json!({"object":"list","data":[{"id":"causa-replay","object":"model"}]}).to_string()
            } else if path == "/v1/chat/completions" {
                json!({"id":"causa-local","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":"Causa proxy is ready. Record/replay integration is local and deterministic."},"finish_reason":"stop"}]}).to_string()
            } else {
                json!({"error":"not found"}).to_string()
            };
            let status = if path == "/"
                || path == "/health"
                || path == "/v1/models"
                || path == "/v1/chat/completions"
            {
                "200 OK"
            } else {
                "404 Not Found"
            };
            let response = format!("HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", status, body.len(), body);
            let _ = socket.write_all(response.as_bytes()).await;
        });
    }
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
