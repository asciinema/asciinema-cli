use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::fs;
use std::io::BufRead;
use std::io::{self, Write};
use std::path::Path;

pub struct Reader<'a> {
    pub header: Header,
    pub events: Box<dyn Iterator<Item = Result<Event>> + 'a>,
}

pub struct Writer<W: Write> {
    writer: io::LineWriter<W>,
    time_offset: u64,
}

pub struct Header {
    pub version: u8,
    pub cols: u16,
    pub rows: u16,
    pub timestamp: Option<u64>,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct V1 {
    version: u8,
    width: u16,
    height: u16,
    command: Option<String>,
    title: Option<String>,
    env: Option<HashMap<String, String>>,
    stdout: Vec<V1Stdout>,
}

#[derive(Debug, Deserialize)]
struct V1Stdout {
    #[serde(deserialize_with = "deserialize_time")]
    time: u64,
    data: String,
}

#[derive(Deserialize)]
pub struct V2Header {
    pub version: u8,
    pub width: u16,
    pub height: u16,
    pub timestamp: Option<u64>,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(deserialize_with = "deserialize_time")]
    pub time: u64,
    #[serde(deserialize_with = "deserialize_code")]
    pub code: EventCode,
    pub data: String,
}

#[derive(PartialEq, Debug)]
pub enum EventCode {
    Output,
    Input,
    Resize,
    Marker,
    Other(char),
}

impl<W> Writer<W>
where
    W: Write,
{
    pub fn new(writer: W, time_offset: u64) -> Self {
        Self {
            writer: io::LineWriter::new(writer),
            time_offset,
        }
    }

    pub fn write_header(&mut self, header: &Header) -> io::Result<()> {
        let header: V2Header = header.into();
        writeln!(self.writer, "{}", serde_json::to_string(&header)?)
    }

    pub fn write_event(&mut self, mut event: Event) -> io::Result<()> {
        event.time += self.time_offset;

        writeln!(self.writer, "{}", serialize_event(&event)?)
    }
}

pub fn get_duration<S: AsRef<Path>>(path: S) -> Result<u64> {
    let Reader { events, .. } = open_from_path(path)?;
    let time = events.last().map_or(Ok(0), |e| e.map(|e| e.time))?;

    Ok(time)
}

pub fn open_from_path<S: AsRef<Path>>(path: S) -> Result<Reader<'static>> {
    fs::File::open(path)
        .map(io::BufReader::new)
        .map_err(|e| anyhow!(e))
        .and_then(open)
        .map_err(|e| anyhow!("can't open asciicast file: {e}"))
}

pub fn open<'a, R: BufRead + 'a>(reader: R) -> Result<Reader<'a>> {
    let mut lines = reader.lines();
    let first_line = lines.next().ok_or(anyhow!("empty file"))??;

    if let Ok(header) = serde_json::from_str::<V2Header>(&first_line) {
        if header.version != 2 {
            bail!("unsupported asciicast version")
        }

        let header: Header = header.into();
        let events = Box::new(lines.filter_map(parse_event));

        Ok(Reader { header, events })
    } else {
        let json = std::iter::once(Ok(first_line))
            .chain(lines)
            .collect::<io::Result<String>>()?;

        let asciicast: V1 = serde_json::from_str(&json)?;

        if asciicast.version != 1 {
            bail!("unsupported asciicast version")
        }

        let header: Header = (&asciicast).into();

        let events = Box::new(
            asciicast
                .stdout
                .into_iter()
                .map(|e| Ok(Event::output(e.time, e.data.as_bytes()))),
        );

        Ok(Reader { header, events })
    }
}

fn parse_event(line: io::Result<String>) -> Option<Result<Event>> {
    match line {
        Ok(line) => {
            if line.is_empty() {
                None
            } else {
                Some(serde_json::from_str(&line).map_err(|e| e.into()))
            }
        }

        Err(e) => Some(Err(e.into())),
    }
}

fn deserialize_time<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    let string = value.as_f64().map(|v| v.to_string()).unwrap_or_default();
    let parts: Vec<&str> = string.split('.').collect();

    match parts.as_slice() {
        [left, right] => {
            let secs: u64 = left.parse().map_err(Error::custom)?;

            let right = right.trim();
            let micros: u64 = format!("{:0<6}", &right[..(6.min(right.len()))])
                .parse()
                .map_err(Error::custom)?;

            Ok(secs * 1_000_000 + micros)
        }

        _ => Err(Error::custom("invalid time format")),
    }
}

fn deserialize_code<'de, D>(deserializer: D) -> Result<EventCode, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use EventCode::*;

    let value: &str = Deserialize::deserialize(deserializer)?;

    match value {
        "o" => Ok(Output),
        "i" => Ok(Input),
        "r" => Ok(Resize),
        "m" => Ok(Marker),
        "" => Err(Error::custom("missing event code")),
        s => Ok(Other(s.chars().next().unwrap())),
    }
}

impl Event {
    pub fn output(time: u64, data: &[u8]) -> Self {
        Event {
            time,
            code: EventCode::Output,
            data: String::from_utf8_lossy(data).to_string(),
        }
    }

    pub fn input(time: u64, data: &[u8]) -> Self {
        Event {
            time,
            code: EventCode::Input,
            data: String::from_utf8_lossy(data).to_string(),
        }
    }

    pub fn resize(time: u64, size: (u16, u16)) -> Self {
        Event {
            time,
            code: EventCode::Resize,
            data: format!("{}x{}", size.0, size.1),
        }
    }

    pub fn marker(time: u64) -> Self {
        Event {
            time,
            code: EventCode::Marker,
            data: "".to_owned(),
        }
    }
}

impl Display for EventCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        use EventCode::*;

        match self {
            Output => f.write_str("o"),
            Input => f.write_str("i"),
            Resize => f.write_str("r"),
            Marker => f.write_str("m"),
            Other(t) => f.write_str(&t.to_string()),
        }
    }
}

impl serde::Serialize for V2Header {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut len = 4;

        if self.idle_time_limit.is_some() {
            len += 1;
        }

        if self.command.is_some() {
            len += 1;
        }

        if self.title.is_some() {
            len += 1;
        }

        if self.env.as_ref().is_some_and(|env| !env.is_empty()) {
            len += 1;
        }

        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("version", &self.version)?;
        map.serialize_entry("width", &self.width)?;
        map.serialize_entry("height", &self.height)?;
        map.serialize_entry("timestamp", &self.timestamp)?;

        if let Some(limit) = self.idle_time_limit {
            map.serialize_entry("idle_time_limit", &limit)?;
        }

        if let Some(command) = &self.command {
            map.serialize_entry("command", &command)?;
        }

        if let Some(title) = &self.title {
            map.serialize_entry("title", &title)?;
        }

        if let Some(env) = &self.env {
            if !env.is_empty() {
                map.serialize_entry("env", &env)?;
            }
        }

        map.end()
    }
}

impl From<&Header> for V2Header {
    fn from(header: &Header) -> Self {
        V2Header {
            version: 2,
            width: header.cols,
            height: header.rows,
            timestamp: header.timestamp,
            idle_time_limit: header.idle_time_limit,
            command: header.command.clone(),
            title: header.title.clone(),
            env: header.env.clone(),
        }
    }
}

impl From<V2Header> for Header {
    fn from(header: V2Header) -> Self {
        Header {
            version: 2,
            cols: header.width,
            rows: header.height,
            timestamp: None,
            idle_time_limit: None,
            command: header.command,
            title: header.title,
            env: header.env,
        }
    }
}

impl From<&V1> for Header {
    fn from(header: &V1) -> Self {
        Header {
            version: 1,
            cols: header.width,
            rows: header.height,
            timestamp: None,
            idle_time_limit: None,
            command: header.command.clone(),
            title: header.title.clone(),
            env: header.env.clone(),
        }
    }
}

fn serialize_event(event: &Event) -> Result<String, serde_json::Error> {
    Ok(format!(
        "[{}, {}, {}]",
        format_time(event.time).trim_end_matches('0'),
        serde_json::to_string(&event.code.to_string())?,
        serde_json::to_string(&event.data)?
    ))
}

fn format_time(time: u64) -> String {
    format!("{}.{:0>6}", time / 1_000_000, time % 1_000_000)
}

pub fn limit_idle_time(
    events: impl Iterator<Item = Result<Event>>,
    limit: f64,
) -> impl Iterator<Item = Result<Event>> {
    let limit = (limit * 1_000_000.0) as u64;
    let mut prev_time = 0;
    let mut offset = 0;

    events.map(move |event| {
        event.map(|event| {
            let delay = event.time - prev_time;

            if delay > limit {
                offset += delay - limit;
            }

            prev_time = event.time;
            let time = event.time - offset;

            Event { time, ..event }
        })
    })
}

pub fn accelerate(
    events: impl Iterator<Item = Result<Event>>,
    speed: f64,
) -> impl Iterator<Item = Result<Event>> {
    events.map(move |event| {
        event.map(|event| {
            let time = ((event.time as f64) / speed) as u64;

            Event { time, ..event }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{Event, EventCode, Header, Reader, Writer};
    use anyhow::Result;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io;

    #[test]
    fn open_v1_minimal() {
        let file = File::open("tests/casts/minimal.json").unwrap();
        let Reader { header, events } = super::open(io::BufReader::new(file)).unwrap();
        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!(header.version, 1);
        assert_eq!((header.cols, header.rows), (100, 50));

        assert_eq!(events[0].time, 1230000);
        assert_eq!(events[0].code, EventCode::Output);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn open_v1_full() {
        let file = File::open("tests/casts/full.json").unwrap();
        let Reader { header, events } = super::open(io::BufReader::new(file)).unwrap();
        let events = events.collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!(header.version, 1);
        assert_eq!((header.cols, header.rows), (100, 50));

        assert_eq!(events[0].time, 1);
        assert_eq!(events[0].code, EventCode::Output);
        assert_eq!(events[0].data, "ż");

        assert_eq!(events[1].time, 100000);
        assert_eq!(events[1].code, EventCode::Output);
        assert_eq!(events[1].data, "ółć");

        assert_eq!(events[2].time, 10500000);
        assert_eq!(events[2].code, EventCode::Output);
        assert_eq!(events[2].data, "\r\n");
    }

    #[test]
    fn open_v2() {
        let file = File::open("tests/casts/demo.cast").unwrap();
        let Reader { header, events } = super::open(io::BufReader::new(file)).unwrap();
        let events = events.take(7).collect::<Result<Vec<Event>>>().unwrap();

        assert_eq!((header.cols, header.rows), (75, 18));

        assert_eq!(events[1].time, 100989);
        assert_eq!(events[1].code, EventCode::Output);
        assert_eq!(events[1].data, "\u{1b}[?2004h");

        assert_eq!(events[5].time, 1511526);
        assert_eq!(events[5].code, EventCode::Input);
        assert_eq!(events[5].data, "v");

        assert_eq!(events[6].time, 1511937);
        assert_eq!(events[6].code, EventCode::Output);
        assert_eq!(events[6].data, "v");
    }

    #[test]
    fn writer() {
        let mut data = Vec::new();

        {
            let mut fw = Writer::new(&mut data, 0);

            let header = Header {
                version: 2,
                cols: 80,
                rows: 24,
                timestamp: None,
                idle_time_limit: None,
                command: None,
                title: None,
                env: Default::default(),
            };

            fw.write_header(&header).unwrap();
            fw.write_event(Event::output(1000001, "hello\r\n".as_bytes()))
                .unwrap();
        }

        {
            let mut fw = Writer::new(&mut data, 1000001);

            fw.write_event(Event::output(1000001, "world".as_bytes()))
                .unwrap();
            fw.write_event(Event::input(2000002, " ".as_bytes()))
                .unwrap();
            fw.write_event(Event::resize(3000003, (100, 40))).unwrap();
            fw.write_event(Event::output(4000004, "żółć".as_bytes()))
                .unwrap();
        }

        let lines = parse(data);

        assert_eq!(lines[0]["version"], 2);
        assert_eq!(lines[0]["width"], 80);
        assert_eq!(lines[0]["height"], 24);
        assert!(lines[0]["timestamp"].is_null());
        assert_eq!(lines[1][0], 1.000001);
        assert_eq!(lines[1][1], "o");
        assert_eq!(lines[1][2], "hello\r\n");
        assert_eq!(lines[2][0], 2.000002);
        assert_eq!(lines[2][1], "o");
        assert_eq!(lines[2][2], "world");
        assert_eq!(lines[3][0], 3.000003);
        assert_eq!(lines[3][1], "i");
        assert_eq!(lines[3][2], " ");
        assert_eq!(lines[4][0], 4.000004);
        assert_eq!(lines[4][1], "r");
        assert_eq!(lines[4][2], "100x40");
        assert_eq!(lines[5][0], 5.000005);
        assert_eq!(lines[5][1], "o");
        assert_eq!(lines[5][2], "żółć");
    }

    #[test]
    fn write_header() {
        let mut data = Vec::new();

        {
            let mut fw = Writer::new(io::Cursor::new(&mut data), 0);
            let mut env = HashMap::new();
            env.insert("SHELL".to_owned(), "/usr/bin/fish".to_owned());
            env.insert("TERM".to_owned(), "xterm256-color".to_owned());

            let header = Header {
                version: 2,
                cols: 80,
                rows: 24,
                timestamp: Some(1704719152),
                idle_time_limit: Some(1.5),
                command: Some("/bin/bash".to_owned()),
                title: Some("Demo".to_owned()),
                env: Some(env),
            };

            fw.write_header(&header).unwrap();
        }

        let lines = parse(data);

        assert_eq!(lines[0]["version"], 2);
        assert_eq!(lines[0]["width"], 80);
        assert_eq!(lines[0]["height"], 24);
        assert_eq!(lines[0]["timestamp"], 1704719152);
        assert_eq!(lines[0]["idle_time_limit"], 1.5);
        assert_eq!(lines[0]["command"], "/bin/bash");
        assert_eq!(lines[0]["title"], "Demo");
        assert_eq!(lines[0]["env"].as_object().unwrap().len(), 2);
        assert_eq!(lines[0]["env"]["SHELL"], "/usr/bin/fish");
        assert_eq!(lines[0]["env"]["TERM"], "xterm256-color");
    }

    fn parse(json: Vec<u8>) -> Vec<serde_json::Value> {
        String::from_utf8(json)
            .unwrap()
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(serde_json::from_str::<serde_json::Value>)
            .collect::<serde_json::Result<Vec<_>>>()
            .unwrap()
    }

    #[test]
    fn accelerate() {
        let stdout = [(0u64, "foo"), (20, "bar"), (50, "baz")]
            .map(|(time, output)| Ok(Event::output(time, output.as_bytes())));

        let stdout = super::accelerate(stdout.into_iter(), 2.0)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(stdout[0].time, 0);
        assert_eq!(stdout[0].data, "foo");
        assert_eq!(stdout[1].time, 10);
        assert_eq!(stdout[1].data, "bar");
        assert_eq!(stdout[2].time, 25);
        assert_eq!(stdout[2].data, "baz");
    }

    #[test]
    fn limit_idle_time() {
        let stdout = [
            (0, "foo"),
            (1_000_000, "bar"),
            (3_500_000, "baz"),
            (4_000_000, "qux"),
            (7_500_000, "quux"),
        ]
        .map(|(time, output)| Ok(Event::output(time, output.as_bytes())));

        let stdout = super::limit_idle_time(stdout.into_iter(), 2.0)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(stdout[0].time, 0);
        assert_eq!(stdout[0].data, "foo");
        assert_eq!(stdout[1].time, 1_000_000);
        assert_eq!(stdout[1].data, "bar");
        assert_eq!(stdout[2].time, 3_000_000);
        assert_eq!(stdout[2].data, "baz");
        assert_eq!(stdout[3].time, 3_500_000);
        assert_eq!(stdout[3].data, "qux");
        assert_eq!(stdout[4].time, 5_500_000);
        assert_eq!(stdout[4].data, "quux");
    }
}
