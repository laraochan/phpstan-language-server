use serde_json::Value;
use std::io::{self, BufRead, Write};

pub fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }

    let Some(length) = content_length else {
        return Ok(None);
    };
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(io::Error::other)
}

pub fn write_message(output: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::{read_message, write_message};
    use serde_json::json;
    use std::io::{BufReader, Cursor};

    #[test]
    fn round_trips_lsp_message() {
        let message = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
        let mut encoded = Vec::new();
        write_message(&mut encoded, &message).unwrap();

        let decoded = read_message(&mut BufReader::new(Cursor::new(encoded)))
            .unwrap()
            .unwrap();
        assert_eq!(decoded, message);
    }
}
