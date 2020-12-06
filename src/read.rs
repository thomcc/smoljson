// use crate::value::Num;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
/// First lifetime is for strings borrowed from the source.
/// Second lifetime is for strings borrowed from the parser.
#[derive(PartialEq, Debug, Clone)]
pub(crate) enum Token<'s> {
    Null,
    Bool(bool),
    NumU(u64),
    NumI(i64),
    NumF(f64),
    StrBorrow(&'s str),
    StrOwn(Box<str>),
    Colon,
    Comma,
    ObjectBegin,
    ObjectEnd,
    ArrayBegin,
    ArrayEnd,
}

// impl<'p> Token<'p> {
//     pub fn get_bool(&self) -> Option<bool> {
//         match self {
//             Self::Bool(b) => Some(*b),
//             _ => None,
//         }
//     }
//     pub fn get_str(&self) -> Option<&str> {
//         match self {
//             Self::StrBorrow(s) => Some(*s),
//             Self::StrOwn(s) => Some(&*s),
//             _ => None,
//         }
//     }
//     pub fn get_f64(&self) -> Option<f64> {
//         match self {
//             Self::NumU(n) => Some(*n as f64),
//             Self::NumI(n) => Some(*n as f64),
//             Self::NumF(n) => Some(*n),
//             _ => None,
//         }
//     }
//     pub fn get_int<V: TryFrom<i128>>(&self) -> Option<V> {
//         match self {
//             Self::NumU(n) => V::try_from(*n as i128).ok(),
//             Self::NumI(n) => V::try_from(*n as i128).ok(),
//             Self::NumF(n) => V::try_from(*n as i128).ok(),
//             _ => None,
//         }
//     }
// }

#[derive(Debug, Clone)]
pub struct Error {
    #[cfg(any(debug_assertions, feature = "better_errors"))]
    _pos: (usize, usize, usize),
    #[cfg(not(any(debug_assertions, feature = "better_errors")))]
    _priv: (),
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(any(debug_assertions, feature = "better_errors"))]
        {
            write!(
                f,
                "JSON parse error around index {} (line {} column {})",
                self._pos.0, self._pos.1, self._pos.2
            )
        }
        #[cfg(not(any(debug_assertions, feature = "better_errors")))]
        {
            f.write_str("JSON parse error")
        }
    }
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub struct Reader<'a> {
    input: &'a str,
    bytes: &'a [u8],
    tok_start: usize,
    pos: usize,
    buf: String,
    stash: Option<Token<'a>>,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input,
            bytes: input.as_bytes(),
            pos: 0,
            buf: "".into(),
            tok_start: 0,
            stash: None,
        }
    }

    #[cold]
    pub(super) fn err(&mut self) -> Error {
        #[cfg(any(debug_assertions, feature = "better_errors"))]
        {
            let index = self.pos.min(self.input.len());
            // note: use `bytes` to avoid panic if index not on char_boundary.
            let so_far = &self.bytes[..index];
            let line = so_far.iter().filter(|n| **n == b'\n').count();
            // byte index isn't ideal for column, but eh.
            let col = if line == 0 {
                index
            } else {
                so_far
                    .iter()
                    .rposition(|n| *n == b'\n')
                    .map(|i| i + 1)
                    .unwrap_or_default()
            };
            Error {
                _pos: (index, line, col),
            }
        }
        #[cfg(not(any(debug_assertions, feature = "better_errors")))]
        {
            Error { _priv: () }
        }
    }

    fn bnext(&mut self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            let ch = self.bytes[self.pos];
            self.pos += 1;
            Some(ch)
        } else {
            None
        }
    }

    fn bnext_or_err(&mut self) -> Result<u8> {
        match self.bnext() {
            Some(c) => Ok(c),
            None => Err(self.err()),
        }
    }

    fn bpeek(&mut self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            Some(self.bytes[self.pos])
        } else {
            None
        }
    }

    fn bpeek_or_nul(&mut self) -> u8 {
        self.bpeek().unwrap_or(b'\0')
    }

    fn bump(&mut self) {
        self.pos += 1;
        debug_assert!(self.pos <= self.input.len());
    }

    fn finished(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    pub(super) fn ref_stash(&self) -> Option<&Token<'a>> {
        self.stash.as_ref()
    }

    pub(super) fn mut_stash(&mut self) -> &mut Option<Token<'a>> {
        &mut self.stash
    }
    pub(super) fn take_stash(&mut self) -> Option<Token<'a>> {
        self.stash.take()
    }
    pub(super) fn skipnpeek(&mut self) -> Option<u8> {
        debug_assert!(self.stash.is_none());
        self.skip_ws();
        self.bpeek()
    }

    fn skip_ws(&mut self) {
        let (mut p, bs) = (self.pos, self.bytes);
        while p < bs.len() && matches!(bs[p], b'\n' | b' ' | b'\t' | b'\r') {
            p += 1;
        }
        self.pos = p;
    }

    fn cur_ch(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn single_hex_escape(&mut self) -> Result<u16> {
        let mut acc = 0;
        for _ in 0..4 {
            let b = tri!(self.bnext_or_err());
            let n = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(self.err()),
            };
            acc = acc * 16 + (n as u16);
        }
        Ok(acc)
    }

    fn read_hex_escape(&mut self) -> Result<()> {
        // todo: option where we reutrn an error (instead using replacement
        // char) if unescaping produces unpaired surrogates.
        use core::char::REPLACEMENT_CHARACTER as REPLACEMENT;
        const LEAD: core::ops::Range<u16> = 0xd800..0xdc00;
        const TRAIL: core::ops::Range<u16> = 0xdc00..0xe000;

        let lead = tri!(self.single_hex_escape());
        if let Some(c) = core::char::from_u32(lead as u32) {
            self.buf.push(c);
            return Ok(());
        }
        if TRAIL.contains(&lead) {
            self.buf.push(REPLACEMENT);
            return Ok(());
        }
        debug_assert!(LEAD.contains(&lead));
        let p = self.pos;
        let trail = if self.bytes[p..].starts_with(b"\\u") {
            self.pos += 2;
            tri!(self.single_hex_escape())
        } else {
            self.buf.push(REPLACEMENT);
            return Ok(());
        };
        if !TRAIL.contains(&trail) {
            // rewind here so we follow algorithm 2 (max subparts of illegal
            // sequence) for https://www.unicode.org/review/pr-121.html.
            self.pos = p;
            self.buf.push(REPLACEMENT);
            return Ok(());
        }
        let scalar = (((lead as u32 - 0xd800) << 10) | (trail as u32 - 0xdc00)) + 0x10000;
        debug_assert!(
            core::char::from_u32(scalar).is_some(),
            r#""\u{:04x}\u{:04x}" => {:#x}"#,
            lead,
            trail,
            scalar,
        );
        // all well-formed surrogate pairs map to `char`s (e.g. unicode scalar
        // values), so unwrap is fine
        self.buf.push(core::char::from_u32(scalar).unwrap());
        Ok(())
    }

    fn expect_next(&mut self, next: &[u8]) -> Result<()> {
        for &i in next {
            if Some(i) != self.bnext() {
                return Err(self.err());
            }
        }
        Ok(())
    }

    fn unescape_next(&mut self) -> Result<()> {
        let b = tri!(self.bnext_or_err());
        match b {
            b'b' => self.buf.push('\x08'),
            b'f' => self.buf.push('\x0c'),
            b'n' => self.buf.push('\n'),
            b'r' => self.buf.push('\r'),
            b't' => self.buf.push('\t'),
            b'\\' => self.buf.push('\\'),
            b'/' => self.buf.push('/'),
            b'\"' => self.buf.push('\"'),
            b'u' => return self.read_hex_escape(),
            _ => return Err(self.err()),
        }
        Ok(())
    }

    fn read_keyword(&mut self, id: &[u8], t: Token<'a>) -> Result<Token<'a>> {
        debug_assert_eq!(self.bytes[self.pos - 1], id[0]);
        tri!(self.expect_next(&id[1..]));
        Ok(t)
    }

    pub(crate) fn unpeek(&mut self, t: Token<'a>) {
        assert!(self.stash.is_none());
        self.stash = Some(t);
    }
    pub(crate) fn next_token(&mut self) -> Result<Option<Token<'a>>> {
        if let Some(t) = self.stash.take() {
            return Ok(Some(t));
        }
        self.skip_ws();
        if self.pos >= self.input.len() {
            return Ok(None);
        }
        self.tok_start = self.pos;
        let tok = match tri!(self.bnext_or_err()) {
            b':' => return Ok(Some(Token::Colon)),
            b',' => return Ok(Some(Token::Comma)),
            b'{' => return Ok(Some(Token::ObjectBegin)),
            b'}' => return Ok(Some(Token::ObjectEnd)),
            b'[' => return Ok(Some(Token::ArrayBegin)),
            b']' => return Ok(Some(Token::ArrayEnd)),
            b'"' => self.read_string(),
            b't' => self.read_keyword(b"true", Token::Bool(true)),
            b'f' => self.read_keyword(b"false", Token::Bool(false)),
            b'n' => self.read_keyword(b"null", Token::Null),
            b'-' | b'0'..=b'9' => self.read_num(),
            _ => return Err(self.err()),
        };
        Ok(Some(tri!(tok)))
    }

    fn is_delim_byte(&self, b: u8) -> bool {
        matches!(b, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
    }

    fn read_num(&mut self) -> Result<Token<'a>> {
        let neg = self.bytes[self.tok_start] == b'-';
        let mut float = false;
        while let Some(b) = self.bpeek() {
            match b {
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    float = true;
                    self.bump();
                }
                b'0'..=b'9' => {
                    self.bump();
                }
                b if self.is_delim_byte(b) => break,
                _ => return Err(self.err()),
            }
        }
        let text = &self.input[self.tok_start..self.pos];
        if !float {
            if neg {
                if let Ok(i) = text.parse::<i64>() {
                    debug_assert!(i < 0);
                    return Ok(Token::NumI(i));
                }
            } else if let Ok(u) = text.parse::<u64>() {
                return Ok(Token::NumU(u));
            }
        }
        if let Ok(v) = text.parse::<f64>() {
            Ok(Token::NumF(v))
        } else {
            Err(self.err())
        }
    }

    fn read_string(&mut self) -> Result<Token<'a>> {
        self.buf.clear();
        let bs = self.bytes;
        loop {
            let mut p = self.pos;
            let start = p;
            while p < bs.len() && bs[p] != b'"' && bs[p] != b'\\' {
                p += 1;
            }
            if p == bs.len() || !self.input.is_char_boundary(p) {
                self.pos = p;
                return Err(self.err());
            }
            self.pos = p + 1;
            if bs[p] == b'"' && self.buf.is_empty() {
                // didn't need any unescaping.
                return Ok(Token::StrBorrow(&self.input[start..p]));
            }
            self.buf.push_str(&self.input[start..p]);
            if bs[p] == b'"' {
                return Ok(Token::StrOwn(self.buf.clone().into_boxed_str()));
            }
            debug_assert_eq!(bs[p], b'\\');
            tri!(self.unescape_next());
        }
    }
}

macro_rules! tok_tester {
    ($($func:ident matches $tok:ident);*) => {$(
        pub(crate) fn $func(&mut self) -> Result<()> {
            match self.next_token() {
                Ok(Some(Token::$tok)) => Ok(()),
                Err(e) => Err(e),
                _ => Err(self.err()),
            }
        }
    )*};
}
impl<'a> Reader<'a> {
    pub(crate) fn next(&mut self) -> Result<Token<'a>> {
        match self.next_token() {
            Ok(Some(v)) => Ok(v),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    tok_tester! {
        array_begin matches ArrayBegin;
        // array_end matches ArrayEnd;
        obj_begin matches ObjectBegin;
        // obj_end matches ObjectEnd;
        comma matches Comma;
        colon matches Colon;
        null matches Null
    }
    pub(crate) fn comma_or_obj_end(&mut self) -> Result<bool> {
        match self.next_token() {
            Ok(Some(Token::Comma)) => Ok(true),
            Ok(Some(Token::ObjectEnd)) => Ok(false),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    pub(crate) fn comma_or_array_end(&mut self) -> Result<bool> {
        match self.next_token() {
            Ok(Some(Token::Comma)) => Ok(true),
            Ok(Some(Token::ArrayEnd)) => Ok(false),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    pub(crate) fn key(&mut self) -> Result<Cow<'a, str>> {
        match self.next_token() {
            Ok(Some(Token::StrBorrow(b))) => Ok(Cow::Borrowed(b)),
            Ok(Some(Token::StrOwn(b))) => Ok(Cow::Owned(b.into())),
            Err(e) => Err(e),
            Ok(Some(_t)) => {
                return Err(self.err());
            }
            _o => return Err(self.err()),
        }
    }
}

fn dec_utf16_single(a: u16, b: u16) -> Option<char> {
    if (0xdc00..=0xdfff).contains(&a) || !(0xdc00..=0xdfff).contains(&b) {
        return None;
    }
    debug_assert!((0xd800..0xdc00).contains(&a), "huh? {:#x}", a);
    let c = (((a as u32 - 0xd800) << 10) | (b as u32 - 0xdc00)) + 0x10000;
    core::char::from_u32(c)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_u16() {
        for c in (0x10000..0x110000).filter_map(core::char::from_u32) {
            let mut buf = [0, 0];
            c.encode_utf16(&mut buf);
            assert_eq!(dec_utf16_single(buf[0], buf[1]), Some(c));
        }
    }
}

impl<'a> Reader<'a> {
    // pub fn peek_bool(&mut self) -> Result<bool> {
    //     if let Ok(Some(Token::Bool(b))) = self.peek_token() {
    //         Ok(*b)
    //     } else {
    //         Err(self.err())
    //     }
    // }
    // pub fn peek_f64(&mut self) -> Result<f64> {
    //     match self.peek_token() {
    //         Ok(Some(Token::NumF(f))) => Ok(*f as f64),
    //         Ok(Some(Token::NumI(i))) => Ok(*i as f64),
    //         Ok(Some(Token::NumU(i))) => Ok(*i as f64),
    //         Err(e) => Err(e),
    //         _ => Err(self.err()),
    //     }
    // }
    pub fn read_i64(&mut self) -> Result<i64> {
        match self.next_token() {
            Ok(Some(Token::NumF(f))) => Ok(f as i64),
            Ok(Some(Token::NumI(i))) => Ok(i),
            Ok(Some(Token::NumU(i))) => Ok(i as i64),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    pub fn read_u64(&mut self) -> Result<u64> {
        match self.next_token() {
            Ok(Some(Token::NumF(f))) => Ok(f as u64),
            Ok(Some(Token::NumI(i))) => Ok(i as f64 as u64),
            Ok(Some(Token::NumU(i))) => Ok(i),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    pub fn read_str(&mut self) -> Result<Cow<'a, str>> {
        match self.next_token() {
            Ok(Some(Token::StrBorrow(s))) => Ok(Cow::Borrowed(s)),
            Ok(Some(Token::StrOwn(s))) => Ok(Cow::Owned(s.into())),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    // pub fn read_object(&mut self) -> Result<()> {}
}

// pub trait ReadJson {
//     fn read_json(&mut self, w: &mut Reader) -> Result<()>;
// }
// impl ReadJson for f64 {
//     fn read_json(&mut self, w: &mut Reader) {
//         w.next()
//     }
// }

/*

#[derive(Copy, Clone, Debug, Default)]
pub struct Null;

#[derive(Copy, Clone, Debug, Default)]
pub struct Undefined;

impl WriteJson for Undefined {
    fn write_json(&self, dest: &mut Writer) {
        debug_assert!(false);
        Null.write_json(dest)
    }
    fn should_include(&self) -> bool {
        false
    }
}

impl WriteJson for Null {
    fn write_json(&self, dest: &mut Writer) {
        dest.o.push_str("null");
    }
}

impl WriteJson for f64 {
    fn write_json(&self, dest: &mut Writer) {
        if self.is_nan() {
            Undefined.write_json(dest);
        } else if self.is_infinite() {
            let max = if *self < 0.0 { -f64::MAX } else { f64::MAX };
            let _ = write!(&mut dest.o, "{}", max);
        } else {
            let _ = write!(&mut dest.o, "{}", self);
        }
    }
    fn should_include(&self) -> bool {
        !self.is_nan()
    }
}

impl WriteJson for str {
    fn write_json(&self, dest: &mut Writer) {
        dest.put_escaped(self, true)
    }
}
impl<T: ?Sized + WriteJson> WriteJson for &T {
    fn write_json(&self, dest: &mut Writer) {
        T::write_json(*self, dest)
    }
}

impl WriteJson for [(&str, &dyn WriteJson)] {
    fn write_json(&self, dest: &mut Writer) {
        dest.put_object(self);
    }
}

impl<T: WriteJson> WriteJson for [T] {
    fn write_json(&self, dest: &mut Writer) {
        let mut a = dest.array();
        for i in self.iter() {
            a.put(i);
        }
    }
}

impl<T: WriteJson> WriteJson for Option<T> {
    fn write_json(&self, dest: &mut Writer) {
        if let Some(v) = self {
            v.write_json(dest);
        } else {
            Undefined.write_json(dest);
        }
    }
    fn should_include(&self) -> bool {
        self.as_ref().map_or(false, T::should_include)
    }
}
*/
