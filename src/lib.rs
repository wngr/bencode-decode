use std::{collections::BTreeMap, io::Read, result::Result};
// Bencoding spec
// https://wiki.theory.org/index.php/BitTorrentSpecification#Bencoding

#[derive(PartialEq, Ord, PartialOrd, Eq, Debug, Clone)]
pub enum Value {
    ByteString(Vec<u8>),
    Integer(i64),
    List(Vec<Value>),
    Dictionary(BTreeMap<Vec<u8>, Value>),
}

#[derive(PartialEq, Debug)]
pub enum ParseResult {
    ValueType(Value),
    ListStart,
    DictStart,
    End,
    EOF,
}

/// Constructs a `Parser` for bencoded data from a reader implementing
/// `std::io::read`. The only exposed interface is an iterator, which
/// will emit parsed tokens `ParseResult` up until (but not including)
/// EOF.
///
/// ```
/// use bencode_decode::Parser;
/// let input = std::io::Cursor::new(
///            "d9:publisher3:bob17:publisher-webpage15:www.example.com18:publisher.location4:homee"
///                .to_string()
///                .into_bytes(),
///        );
/// let mut parser = Parser::new(input);
/// for item in parser {
///     println!("{:?}", item);
/// }
/// ```
pub struct Parser<R: Read> {
    reader: R,
}
impl<R: Read> Parser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: Read> Iterator for Parser<R> {
    type Item = ParseResult;
    fn next(&mut self) -> Option<Self::Item> {
        let res = parse(&mut self.reader).ok();
        if res == Some(ParseResult::EOF) {
            None
        } else {
            res
        }
    }
}

use ParseResult::*;
use Value::*;
/// Given a token parser `parser`, will try to decode `ParseResult` into
/// `Value`s. This function does obviously not attempt to drain the passed
/// reader instance, but rather expects one top-level value to parse form.
///
/// ```
/// use bencode_decode::{Parser, decode};
/// use std::fs::File;
///
/// let f = File::open("./test/ubuntu-18.04.4-live-server-amd64.iso.torrent").unwrap();
/// let mut parser = Parser::new(f);
/// let res = decode(&mut parser, None).unwrap();
/// ```
pub fn decode<R: Read>(parser: &mut Parser<R>, current: Option<ParseResult>) -> Option<Value> {
    match current.or_else(|| parser.next()) {
        Some(ValueType(val)) => Some(val),
        Some(t @ DictStart) | Some(t @ ListStart) => {
            let mut data = vec![];
            let mut next = parser.next().expect("Unexpected EOF");
            while next != End {
                data.push(decode(parser, Some(next)).unwrap());
                next = parser.next().expect("Unexpected EOF");
            }
            if t == ListStart {
                Some(Value::List(data))
            } else {
                let mut map = BTreeMap::new();
                let mut input = data.into_iter();
                while let (Some(ByteString(key)), Some(value)) = (input.next(), input.next()) {
                    map.insert(key, value);
                }
                Some(Dictionary(map))
            }
        }
        Some(End) => unreachable!(),
        Some(EOF) => unreachable!(),
        None => None,
    }
}

fn parse<R: Read>(reader: &mut R) -> Result<ParseResult, Box<dyn std::error::Error>> {
    let mut buf = [0; 1];
    let mut vec = vec![];
    loop {
        let read_bytes = reader.read(&mut buf)?;
        if read_bytes == 0 {
            return Ok(EOF);
        }
        match buf[0] {
            n @ b'0'..=b'9' => vec.push(n),
            b':' => {
                let size = String::from_utf8(vec)?.parse()?;
                let mut str = vec![0; size];
                reader.read_exact(&mut str)?;
                return Ok(ValueType(ByteString(str)));
            }
            b'i' => {
                let mut b = [0; 1];
                reader.read_exact(&mut b)?;
                let mut vec = vec![];
                while b[0] != b'e' {
                    vec.push(b[0]);
                    reader.read_exact(&mut b)?;
                }
                let int: i64 = String::from_utf8(vec)?.parse()?;
                return Ok(ValueType(Integer(int)));
            }
            b'e' => return Ok(End),
            b'l' => return Ok(ListStart),
            b'd' => return Ok(DictStart),
            _ => unreachable!("unexpected token"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    #[test]
    fn torrent() {
        let f = File::open("./test/ubuntu-18.04.4-live-server-amd64.iso.torrent").unwrap();
        let mut parser = Parser::new(f);
        let res = decode(&mut parser, None).unwrap();
        if let Value::Dictionary(x) = res {
            if let Value::Dictionary(y) = x.get(&b"info".to_vec()).unwrap() {
                let path = y.get(&b"name".to_vec()).unwrap();
                let length = y.get(&b"length".to_vec()).unwrap();
                if let (Value::ByteString(path), Value::Integer(length)) = (path, length) {
                    let path = String::from_utf8_lossy(path);
                    println!("{} -> {} bytes", path, length);
                    assert_eq!(path, "ubuntu-18.04.4-live-server-amd64.iso");
                    assert_eq!(*length, 912_261_120);
                }
            }
        }
    }

    #[test]
    fn spec() {
        let input = std::io::Cursor::new(
            "d9:publisher3:bob17:publisher-webpage15:www.example.com18:publisher.location4:homee"
                .to_string()
                .into_bytes(),
        );
        let mut parser = Parser::new(input);
        let res = decode(&mut parser, None).unwrap();
        let mut map = BTreeMap::new();
        vec![
            ("publisher", "bob"),
            ("publisher-webpage", "www.example.com"),
            ("publisher.location", "home"),
        ]
        .into_iter()
        .for_each(|(k, v)| {
            map.insert(
                k.as_bytes().to_vec(),
                Value::ByteString(v.as_bytes().to_vec()),
            );
        });

        assert_eq!(res, Value::Dictionary(map));
    }
}
