use std::{fmt, path::PathBuf, str::FromStr};

use bytes::Bytes;
use clap::ValueEnum;
use stable_eyre::{eyre::WrapErr, Result};
use tokio::io::AsyncWriteExt;

use crate::client::HashPrefix;

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq)]
pub enum Format {
    Text,
    Binary,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Format::Text => write!(f, "text"),
            Format::Binary => write!(f, "binary"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum OutputMode {
    Stdout,
    SingleFile(PathBuf),
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputMode::Stdout => write!(f, "-"),
            OutputMode::SingleFile(path) => write!(f, "{}", path.display()),
        }
    }
}

impl OutputMode {
    pub fn is_not_stdout(&self) -> bool {
        !matches!(self, OutputMode::Stdout)
    }

    pub fn parse(v: &str) -> Result<Self> {
        if v == "-" {
            return Ok(Self::Stdout);
        }

        let path = PathBuf::from_str(v).wrap_err("Invalid path")?;

        Ok(OutputMode::SingleFile(path))
    }

    pub fn into_writer(self, format: Format) -> Result<Output> {
        match self {
            OutputMode::Stdout => Ok(Output::Stdout(tokio::io::stdout())),
            OutputMode::SingleFile(path) if format == Format::Text => {
                let file = std::fs::File::create(path).wrap_err("Failed to create output file")?;

                let file = tokio::fs::File::from_std(file);
                Ok(Output::File(tokio::io::BufWriter::new(file)))
            }
            OutputMode::SingleFile(path) => {
                let file = std::fs::File::create(path).wrap_err("Failed to create output file")?;

                let file = tokio::fs::File::from_std(file);
                Ok(Output::BinFile(tokio::io::BufWriter::new(file)))
            }
        }
    }
}

pub enum Output {
    Stdout(tokio::io::Stdout),
    File(tokio::io::BufWriter<tokio::fs::File>),
    BinFile(tokio::io::BufWriter<tokio::fs::File>),
}

impl Output {
    pub async fn write(&mut self, prefix: HashPrefix, piece: Bytes) -> Result<()> {
        match self {
            Output::Stdout(s) => {
                for line in lines(&piece) {
                    s.write_all(&prefix)
                        .await
                        .wrap_err("failed to write into stdout")?;
                    s.write_all(line)
                        .await
                        .wrap_err("failed to write into stdout")?;
                }
                s.write_all(&[b'\r', b'\n'])
                    .await
                    .wrap_err("failed to write into stdout")?;
            }
            Output::File(s) => {
                for line in lines(&piece) {
                    s.write_all(&prefix)
                        .await
                        .wrap_err("failed to write into file")?;
                    s.write_all(line)
                        .await
                        .wrap_err("failed to write into file")?;
                }
                s.write_all(&[b'\r', b'\n'])
                    .await
                    .wrap_err("failed to write into file")?;
            }
            Output::BinFile(s) => {
                for (hash, prevalence) in hash_and_prevalance(prefix, &piece) {
                    s.write_all(hash.as_ref())
                        .await
                        .wrap_err("writing to file")?;
                    s.write_u64(prevalence).await.wrap_err("writing to file")?;
                }
            }
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<()> {
        match self {
            Output::Stdout(_) => {}
            Output::File(f) | Output::BinFile(f) => {
                f.flush().await.wrap_err("Flush error")?;
            }
        }

        Ok(())
    }
}

fn hash_and_prevalance(
    prefix: HashPrefix,
    piece: &[u8],
) -> impl Iterator<Item = ([u8; 20], u64)> + '_ {
    fn hexdecode(prefix: HashPrefix, rest: &[u8]) -> [u8; 20] {
        use std::io::Write;

        let mut output = [0; 20];
        let mut hexstring = [0u8; 40];
        hexstring.as_mut().write_all(&prefix).unwrap();
        (&mut hexstring[5..40]).write_all(&rest[0..35]).unwrap();

        hex::decode_to_slice(hexstring, &mut output).expect("valid sha1");
        output
    }

    lines(piece).map(move |it| {
        let hex = &it[0..35];
        let prevalence = it[36..].strip_suffix(b"\r\n").unwrap_or(&it[36..]);
        let prevalence = std::str::from_utf8(prevalence).expect("valid string");
        let prevalence = prevalence.parse().expect("valid number");

        (hexdecode(prefix, hex), prevalence)
    })
}

fn lines(piece: &[u8]) -> impl Iterator<Item = &[u8]> {
    piece.chunk_by(|a, b| !(*a == b'\n' && *b != b'\r'))
}
