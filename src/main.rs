use std::{fmt::Write, time::Duration};

use clap::Parser;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use output::OutputMode;
use stable_eyre::eyre::ErrReport;
use tower::{Service, ServiceExt};

use crate::{output::Format, service::make_service};

mod client;
mod output;
mod ranges;
mod service;

#[derive(Parser)]
#[command(version, about, long_about)]
struct Args {
    /// Output of the program, can be stdout, or file
    #[arg(short, long, default_value_t = OutputMode::Stdout, value_parser = OutputMode::parse)]
    output: OutputMode,

    /// Output format, can be text or binary. Only affects `file` output.
    #[arg(short, long, default_value_t = Format::Text)]
    format: Format,

    /// Whether output should be sorted.
    #[arg(short, long)]
    sorted: bool,
}

#[tokio::main]
async fn main() -> stable_eyre::Result<()> {
    stable_eyre::install()?;
    setup_tracing();

    let Args {
        output,
        format,
        sorted,
    } = Args::parse();

    let service = make_service()?;
    let progress = progress_bar(&output);
    let mut output = output.into_writer(format)?;

    let requests = futures::stream::iter(ranges::all_ranges_iter());

    let responses = requests.map(move |hash| {
        let mut service = service.clone();
        async move { service.ready().await?.call(hash).await }
    });

    let mut responses = if sorted {
        responses.buffer_unordered(1000).boxed()
    } else {
        responses.buffered(1000).boxed()
    };

    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
    tokio::spawn(async move {
        while let Some(result) = responses.next().await {
            if let Some(it) = progress.as_ref() {
                it.inc(1)
            }
            tx.send(result).await.unwrap();
        }
        if let Some(it) = progress.as_ref() {
            it.finish()
        }
        Ok::<_, ErrReport>(())
    });

    let mut recv_buffer = Vec::with_capacity(100);
    loop {
        let received = rx.recv_many(&mut recv_buffer, 100).await;
        if received == 0 {
            break;
        }

        for result in recv_buffer.drain(..) {
            let (prefix, piece) = result?;
            output.write(prefix, piece).await?;
        }
    }

    output.flush().await?;

    // Ignore cleanup
    std::process::exit(0)
}

fn progress_bar(mode: &OutputMode) -> Option<ProgressBar> {
    if !mode.is_not_stdout() {
        return None;
    }
    let progress = ProgressBar::new(ranges::total_len());

    progress.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {percent_precise}% ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));
    progress.enable_steady_tick(Duration::from_millis(250));

    Some(progress)
}

fn setup_tracing() {
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
