use super::read::Result;
use super::read::*;
use super::write::{self, WriteJson, Writer};
use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Num(Num),
    Str(Cow<'a, str>),
    Array(Vec<Value<'a>>),
    Object(BTreeMap<Cow<'a, str>, Value<'a>>),
}
impl<'a> Value<'a> {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Num(_))
    }
    pub fn is_str(&self) -> bool {
        matches!(self, Self::Str(_))
    }
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn as_bool(&self) -> Option<bool> {
        opt_extract!(self, Self::Bool(b) => Some(*b))
    }
    pub fn as_f64(&self) -> Option<f64> {
        opt_extract!(self, Self::Num(n) => n.as_f64())
    }
    pub fn as_i64(&self) -> Option<i64> {
        opt_extract!(self, Self::Num(n) => n.as_i64())
    }
    pub fn as_u64(&self) -> Option<u64> {
        opt_extract!(self, Self::Num(n) => n.as_u64())
    }
    pub fn as_str(&self) -> Option<&str> {
        opt_extract!(self, Self::Str(s) => Some(&*s))
    }
    pub fn as_array(&self) -> Option<&[Value<'a>]> {
        opt_extract!(self, Self::Array(a) => Some(&a[..]))
    }
    pub fn as_object(&self) -> Option<&BTreeMap<Cow<'a, str>, Value<'a>>> {
        opt_extract!(self, Self::Object(o) => Some(o))
    }

    pub fn into_str(self) -> Option<Cow<'a, str>> {
        opt_extract!(self, Self::Str(s) => Some(s))
    }
    pub fn into_array(self) -> Option<Vec<Value<'a>>> {
        opt_extract!(self, Self::Array(a) => Some(a))
    }
    pub fn into_object(self) -> Option<BTreeMap<Cow<'a, str>, Value<'a>>> {
        opt_extract!(self, Self::Object(a) => Some(a))
    }

    pub fn as_mut_array(&mut self) -> Option<&mut Vec<Value<'a>>> {
        opt_extract!(self, Self::Array(a) => Some(a))
    }
    pub fn as_mut_object(&mut self) -> Option<&mut BTreeMap<Cow<'a, str>, Value<'a>>> {
        opt_extract!(self, Self::Object(a) => Some(a))
    }

    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        self.as_object().and_then(|s| s.get(key))
    }
    pub fn at(&self, i: usize) -> Option<&Value<'a>> {
        self.as_array().and_then(|s| s.get(i))
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value<'a>> {
        self.as_mut_object().and_then(|s| s.get_mut(key))
    }
    pub fn at_mut(&mut self, i: usize) -> Option<&mut Value<'a>> {
        self.as_mut_array().and_then(|s| s.get_mut(i))
    }
    pub fn take(&mut self) -> Value {
        core::mem::replace(self, Self::Null)
    }
}

static NULL: Value<'static> = Value::Null;

impl<'a> core::ops::Index<usize> for Value<'a> {
    type Output = Value<'a>;
    fn index(&self, i: usize) -> &Value<'a> {
        self.at(i).unwrap_or(&NULL)
    }
}
impl<'a> core::ops::IndexMut<usize> for Value<'a> {
    fn index_mut(&mut self, i: usize) -> &mut Value<'a> {
        self.at_mut(i).unwrap()
    }
}
impl<'a> core::ops::Index<&str> for Value<'a> {
    type Output = Value<'a>;
    fn index(&self, a: &str) -> &Value<'a> {
        self.get(a).unwrap_or(&NULL)
    }
}
impl<'a> core::ops::IndexMut<&str> for Value<'a> {
    fn index_mut(&mut self, s: &str) -> &mut Value<'a> {
        if self.is_null() {
            *self = Value::Object(BTreeMap::new());
        }
        if let Self::Object(o) = self {
            o.entry(Cow::Owned(s.into())).or_insert(Value::Null)
        } else {
            bad_index(s)
        }
    }
}

#[cold]
#[inline(never)]
fn bad_index(s: &str) -> ! {
    panic!("Attempt to insert key {:?} into non-object json value", s)
}

impl Default for Value<'_> {
    fn default() -> Self {
        Self::Null
    }
}
impl From<bool> for Value<'_> {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}
impl From<String> for Value<'_> {
    fn from(s: String) -> Self {
        Self::Str(s.into())
    }
}
impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Self::Str(Cow::Borrowed(s))
    }
}
impl<'a> From<Cow<'a, str>> for Value<'a> {
    fn from(s: Cow<'a, str>) -> Self {
        Self::Str(s)
    }
}

impl<'a> From<Num> for Value<'a> {
    fn from(n: Num) -> Self {
        Self::Num(n)
    }
}

impl<'a> core::iter::FromIterator<Value<'a>> for Value<'a> {
    fn from_iter<T: IntoIterator<Item = Value<'a>>>(iter: T) -> Self {
        Value::Array(iter.into_iter().collect())
    }
}

impl<'a, S: Into<Cow<'a, str>>> core::iter::FromIterator<(S, Value<'a>)> for Value<'a> {
    fn from_iter<T: IntoIterator<Item = (S, Value<'a>)>>(iter: T) -> Self {
        Value::Object(iter.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

impl<'a> Value<'a> {
    pub fn from_str(s: &'a str) -> Result<Self> {
        Self::from_str_with(s, Dialect::DEFAULT)
    }

    pub fn from_str_with(input: &'a str, d: Dialect) -> Result<Self> {
        let mut de = Reader::with_dialect(input, d);
        let r = tri!(Self::from_reader(&mut de));
        tri!(de.finish());
        Ok(r)
    }

    pub fn into_static(self) -> Value<'static> {
        match self {
            Value::Null => Value::Null,
            Value::Bool(b) => Value::Bool(b),
            Value::Num(n) => Value::Num(n),
            Value::Str(Cow::Owned(s)) => Value::Str(Cow::Owned(s)),
            Value::Str(Cow::Borrowed(s)) => Value::Str(Cow::Owned(s.into())),
            Value::Array(a) => Value::Array(a.into_iter().map(|v| v.into_static()).collect()),
            Value::Object(o) => Value::Object(
                o.into_iter()
                    .map(|(s, v)| {
                        let s: Cow<'static, str> = match s {
                            Cow::Borrowed(s) => Cow::Owned(s.into()),
                            Cow::Owned(s) => Cow::Owned(s),
                        };
                        (s, v.into_static())
                    })
                    .collect(),
            ),
        }
    }
    pub fn from_reader(de: &mut Reader<'a>) -> Result<Self> {
        match tri!(de.next()) {
            Token::Null => Ok(Self::Null),
            Token::Bool(b) => Ok(Self::Bool(b)),
            Token::NumF(b) => Ok(Self::from(b)),
            Token::NumI(b) => Ok(Self::from(b)),
            Token::NumU(b) => Ok(Self::from(b)),
            Token::StrBorrow(b) => Ok(Self::Str(Cow::Borrowed(b))),
            Token::StrOwn(b) => Ok(Self::Str(Cow::Owned(b.into()))),
            Token::ArrayBegin => Self::do_read_array(de),
            Token::ObjectBegin => Self::do_read_obj(de),
            _ => Err(de.err()),
        }
    }
    fn do_read_array(de: &mut Reader<'a>) -> Result<Self> {
        let mut v = alloc::vec![];
        if tri!(de.skipnpeek()) == Some(b']') {
            assert!(matches!(de.next_token(), Ok(Some(Token::ArrayEnd))));
            return Ok(Self::Array(v));
        }
        loop {
            v.push(tri!(Value::from_reader(de)));
            if !tri!(de.comma_or_array_end()) {
                break;
            }
        }
        Ok(Self::Array(v))
    }
    fn do_read_obj(de: &mut Reader<'a>) -> Result<Self> {
        let mut obj = BTreeMap::new();
        if tri!(de.skipnpeek()) == Some(b'}') {
            assert!(matches!(de.next_token(), Ok(Some(Token::ObjectEnd))));
            return Ok(Self::Object(obj));
        }
        loop {
            let k = tri!(de.key());
            tri!(de.colon());
            let val = tri!(Value::from_reader(de));
            obj.insert(k, val);
            if !tri!(de.comma_or_obj_end()) {
                break;
            }
        }
        Ok(Self::Object(obj))
    }
}

impl WriteJson for Num {
    fn write_json(&self, w: &mut Writer) {
        match self.0 {
            N::F(n) => n.write_json(w),
            N::I(n) => n.write_json(w),
            N::U(n) => n.write_json(w),
        }
    }
}
impl WriteJson for Value<'_> {
    fn write_json(&self, w: &mut Writer) {
        match self {
            Self::Null => write::Null.write_json(w),
            Self::Bool(b) => b.write_json(w),
            Self::Num(n) => n.write_json(w),
            Self::Str(s) => (&*s).write_json(w),
            Self::Array(s) => {
                let mut a = w.array();
                for v in s {
                    a.put(v);
                }
            }
            Self::Object(s) => {
                let mut o = w.object();
                for (k, v) in s {
                    o.put(&*k, v);
                }
            }
        }
    }
}
impl Value<'_> {
    pub fn to_string(&self, pretty: bool) -> String {
        let mut w = Writer::new(pretty);
        self.write_json(&mut w);
        w.finish()
    }
}
impl core::str::FromStr for Value<'static> {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        // Not really ideal to do 2 passes...
        Value::from_str(s).map(|v| v.into_static())
    }
}

impl core::fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_string(false))
    }
}

#[derive(Debug, Copy, Clone)]
enum N {
    F(f64),
    // only negative
    I(i64),
    U(u64),
}

const MAX_FLOAT_I: f64 = 9007199254740990.0;

// If this returns true for some float, then we'll consider converting it to an
// equivalent int internally.
fn is_sanely_integral(f: f64) -> bool {
    f.is_finite() && f >= -MAX_FLOAT_I && f <= MAX_FLOAT_I && (f as i64 as f64 == f)
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Num(N);

impl Num {
    pub fn from_u64(v: u64) -> Self {
        Self(N::U(v))
    }
    pub fn from_i64(v: i64) -> Self {
        if v < 0 {
            Self(N::I(v))
        } else {
            Self(N::U(v as u64))
        }
    }
    pub fn from_f64(v: f64) -> Self {
        debug_assert!(v.is_finite());
        if is_sanely_integral(v) {
            let result = if v < 0.0 {
                Self(N::I(v as i64))
            } else {
                Self(N::U(v as u64))
            };
            debug_assert_eq!(result.as_f64(), Some(v));
            result
        } else {
            Self(N::F(v))
        }
    }
    pub fn as_f64(self) -> Option<f64> {
        match self.0 {
            N::F(f) => Some(f),
            N::I(i) if (i as f64 as i64) == i => Some(i as f64),
            N::U(i) if (i as f64 as u64) == i => Some(i as f64),
            _ => None,
        }
    }
    pub fn as_u64(self) -> Option<u64> {
        match self.0 {
            N::F(f) if f >= 0.0 && is_sanely_integral(f) => Some(f as u64),
            N::I(i) if i >= 0 => Some(i as u64),
            N::U(u) => Some(u),
            _ => None,
        }
    }
    pub fn as_i64(self) -> Option<i64> {
        match self.0 {
            N::F(f) if is_sanely_integral(f) => Some(f as i64),
            N::I(i) => Some(i),
            N::U(u) if u < (i64::MAX as u64) => Some(u as i64),
            _ => None,
        }
    }
    pub fn as_int(self) -> i128 {
        match self.0 {
            N::F(f) => f as i128,
            N::I(i) => i as i128,
            N::U(u) => u as i128,
        }
    }
    fn desc_id(self) -> u8 {
        match self.0 {
            N::F(_) => 0,
            N::I(_) => 1,
            N::U(_) => 2,
        }
    }
    pub fn get_float(self) -> Option<f64> {
        if let N::F(f) = self.0 {
            Some(f)
        } else {
            None
        }
    }
    pub fn get_int(self) -> Option<i128> {
        match self.0 {
            N::F(_) => None,
            N::I(i) => Some(i as i128),
            N::U(u) => Some(u as i128),
        }
    }
}
impl core::fmt::Display for Num {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            N::F(n) => n.fmt(f),
            N::I(n) => n.fmt(f),
            N::U(n) => n.fmt(f),
        }
    }
}

impl PartialEq for Num {
    fn eq(&self, o: &Self) -> bool {
        use N::*;
        match (self.0, o.0) {
            (F(a), F(b)) => a == b,
            (I(a), I(b)) => a == b,
            (U(a), U(b)) => a == b,
            (I(i), U(_)) | (U(_), I(i)) => {
                debug_assert!(i < 0, "{}", i);
                false
            }
            (F(f), I(i)) | (I(i), F(f)) => {
                debug_assert!(i < 0, "{}", i);
                if f < 0.0 && f >= -MAX_FLOAT_I && f as i64 as f64 == f {
                    (i as f64) == f || (f as i64) == i
                } else {
                    false
                }
            }
            (F(f), U(i)) | (U(i), F(f)) => {
                if f >= 0.0 && f <= MAX_FLOAT_I && f as u64 as f64 == f {
                    (i as f64) == f || (f as u64) == i
                } else {
                    false
                }
            }
        }
    }
}

macro_rules! impl_into_num {
    (@via($bty:ident, $cast:ident) $($t:ident),*) => {$(
        impl From<$t> for Num {
            fn from(t: $t) -> Self { Num::$cast(t as $bty) }
        }
        impl From<$t> for Value<'_> {
            fn from(t: $t) -> Self { Value::Num(Num::$cast(t as $bty)) }
        }
    )*};
}

// impl_into_num!(i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);
impl_into_num!(@via(i64, from_i64) i8, i16, i32, i64, isize);
impl_into_num!(@via(u64, from_u64) u8, u16, u32, u64, usize);
impl_into_num!(@via(f64, from_f64) f32, f64);
