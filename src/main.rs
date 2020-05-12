use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tui::backend::TermionBackend;
use tui::widgets::{Block, Borders, Gauge};
use tui::Terminal;

fn keyin() -> mpsc::Receiver<Key> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        for c in stdin.keys() {
            sender.send(c.unwrap());
        }
    });
    receiver
}

fn watch(file: std::fs::File) -> mpsc::Receiver<String> {
    let mut f = BufReader::new(file);
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || loop {
        let mut buf = String::new();
        let res = match BufRead::read_line(&mut f, &mut buf) {
            Ok(_) => sender.send(buf),
            Err(_) => return,
        };
        if res.is_err() {
            return;
        }
    });
    return receiver;
}

static USAGE: &'static str = "Usage pb: progressfd target";

fn main() -> Result<(), String> {
    let mut argi = std::env::args();
    let _ = argi.next().ok_or("Did you drop an exec(2) argument?")?;
    let inf = File::open(argi.next().ok_or(USAGE)?).map_err(|x| x.to_string())?;
    let max: u64 = argi.next().ok_or(USAGE)?.parse().unwrap();
    let lineinc = watch(inf);
    let keyinc = keyin();

    let stdout = io::stdout().into_raw_mode().map_err(|x| x.to_string())?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|x| x.to_string())?;

    terminal.clear();

    loop {
        match keyinc.recv_timeout(Duration::from_millis(20)) {
            Ok(k) => match k {
                Key::Char('q') => break,
                _ => {}
            },
            Err(_) => {}
        };
        let pro: u64 = match lineinc.recv_timeout(Duration::from_millis(20)) {
            Err(_) => continue,
            Ok(i) => i.trim_end().parse().unwrap(),
        };
        let percent = (100 as f32 * (pro as f32 / max as f32)) as u16;
        if percent == 100 {
            break;
        }
        terminal
            .draw(|mut f| {
                let size = f.size();
                let guage = Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title("Progress"))
                    .percent(percent);
                f.render_widget(guage, size);
            })
            .unwrap();
    }
    terminal.clear();
    Ok(())
}
