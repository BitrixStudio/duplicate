use anyhow::{Context, Result};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
};

#[derive(Debug, Clone)]
pub enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct OutputEvent {
    pub pane: usize,
    pub stream: StreamKind,
    pub line: String,
}

pub fn spawn_children(
    n: usize,
    cmd: &str,
    common_args: &[String],
    instance_groups: &[Vec<String>],
    tx: mpsc::Sender<OutputEvent>,
) -> Result<Vec<Child>> {
    let mut children: Vec<Child> = Vec::with_capacity(n);

    for i in 0..n {
        let mut full_args = Vec::new();
        full_args.extend(common_args.iter().cloned());
        full_args.extend(instance_groups[i].iter().cloned());

        let mut child = Command::new(cmd)
            .args(&full_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn '{cmd}'"))?;

        if let Some(stdout) = child.stdout.take() {
            spawn_reader(i, StreamKind::Stdout, stdout, tx.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_reader(i, StreamKind::Stderr, stderr, tx.clone());
        }

        children.push(child);
    }

    Ok(children)
}

fn spawn_reader<R: Read + Send + 'static>(
    pane: usize,
    stream: StreamKind,
    reader: R,
    tx: mpsc::Sender<OutputEvent>,
) {
    thread::spawn(move || {
        let mut br = BufReader::new(reader);
        let mut buf = Vec::new();

        loop {
            buf.clear();
            match br.read_until(b'\n', &mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buf)
                        .trim_end_matches(&['\n', '\r'][..])
                        .to_string();
                    let _ = tx.send(OutputEvent {
                        pane,
                        stream: stream.clone(),
                        line,
                    });
                }
                Err(_) => break,
            }
        }
    });
}