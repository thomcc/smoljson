use alloc::string::String;
use core::fmt::Write;

#[derive(Clone, Debug, Default)]
pub struct Writer {
    // pretty: bool,
    o: String,
    indent: usize,
    pretty_stack: usize,
}

pub trait WriteJson {
    fn write_json(&self, dest: &mut Writer);
    // if a value returns false here, we omit the `key: value` pair. false for
    // Undefined and Option::None generally.
    fn should_include(&self) -> bool {
        true
    }
}

macro_rules! impl_write_json_prim_display {
    ($($t:ty),+ $(,)?) => {$(
        impl WriteJson for $t {
            fn write_json(&self, dest: &mut Writer) {
                let _ = write!(&mut dest.o, "{}", *self);
            }
        }
    )+};
}
impl_write_json_prim_display!(bool, i32, i64, u64, usize);

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

impl Writer {
    pub fn new(pretty: bool) -> Self {
        Self {
            pretty_stack: if pretty { 0 } else { 1 },
            ..Self::default()
        }
    }
    pub fn finish(self) -> String {
        self.o
    }
    pub fn pretty(&self) -> bool {
        self.pretty_stack == 0
    }
    pub fn push_compact(&mut self) {
        self.pretty_stack += 1
    }
    pub fn pop_compact(&mut self) {
        self.pretty_stack -= 1
    }
    fn put_escaped<'a>(&'a mut self, s: &str, add_quotes: bool) {
        self.o.reserve(s.len() + 2 * (add_quotes as usize));
        if add_quotes {
            self.o.push('"');
        }
        for c in s.chars() {
            match c {
                '\x08' => self.o.push_str("\\b"),
                '\x0c' => self.o.push_str("\\f"),
                '\n' => self.o.push_str("\\n"),
                '\r' => self.o.push_str("\\r"),
                '\t' => self.o.push_str("\\t"),
                '\\' => self.o.push_str("\\\\"),
                '"' => self.o.push_str("\\\""),
                c if c < ' ' => {
                    self.o.push_str("\\u00");
                    let x = b"012345679abcdef";
                    let n0 = x[(c as u32 as usize) >> 4];
                    let n1 = x[(c as u32 as usize) & 0xf];
                    self.o.push(n0 as char);
                    self.o.push(n1 as char);
                }
                _ => self.o.push(c),
            }
        }
        if add_quotes {
            self.o.push('"');
        }
    }
    fn put_indent(&mut self) {
        if self.pretty() {
            self.put_spaces(self.indent);
        }
    }
    fn put_spaces(&mut self, size: usize) {
        const SP: &str = "                                ";
        const _: [(); 32] = [(); SP.len()];
        self.o.reserve(size);
        let mut n = size;
        while n > 0 {
            self.o.push_str(&SP[..n.min(SP.len())]);
            n = n.saturating_sub(SP.len());
        }
    }
    fn ppush<'a>(&'a mut self, if_pretty: &str, if_not_pretty: &str) {
        self.o.push_str(if self.pretty() {
            if_pretty
        } else {
            if_not_pretty
        });
    }
    fn comma_nl(&mut self) {
        self.ppush(",\n", ",");
    }
    fn nl(&mut self) {
        if self.pretty() {
            self.o.push('\n');
        }
    }
    pub fn object(&mut self) -> ObjectWriter<'_> {
        ObjectWriter(SeqWriter::begin(self, false))
    }
    pub fn array(&mut self) -> ArrayWriter<'_> {
        ArrayWriter(SeqWriter::begin(self, true))
    }
    pub fn put_object(&mut self, arr: &[(&str, &dyn WriteJson)]) {
        let mut ow = self.object();
        ow.put_all(arr);
    }
    pub fn put_array(&mut self, arr: &[&dyn WriteJson]) {
        let mut aw = self.array();
        aw.put_all(arr);
    }
    pub fn put_iter<I>(&mut self, it: I)
    where
        I: IntoIterator,
        I::Item: WriteJson,
    {
        let mut aw = self.array();
        aw.put_iter(it);
    }
}

pub struct ArrayWriter<'a>(SeqWriter<'a>);

impl<'a> ArrayWriter<'a> {
    // pub fn writer(&mut self) -> &mut Writer {
    //     self.0.w
    // }
    pub fn begin_object(&mut self) -> ObjectWriter<'_> {
        self.0.enter_key(None);
        ObjectWriter(SeqWriter::begin(self.0.w, false))
    }
    pub fn begin_array(&mut self) -> ArrayWriter<'_> {
        self.0.enter_key(None);
        ArrayWriter(SeqWriter::begin(self.0.w, true))
    }
    pub fn set_compact(&mut self) -> &mut Self {
        self.0.set_compact();
        self
    }
    pub fn compact(mut self) -> Self {
        self.set_compact();
        self
    }
    pub fn put<V: WriteJson>(&mut self, val: V) -> &mut Self {
        self.0.put_impl(None, &val);
        self
    }
    pub fn put_all(&mut self, vals: &[&dyn WriteJson]) -> &mut Self {
        for val in vals {
            self.0.put_impl(None, val);
        }
        self
    }
    pub fn put_iter<I>(&mut self, vals: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: WriteJson,
    {
        for val in vals {
            self.0.put_impl(None, &val);
        }
        self
    }
}

pub struct ObjectWriter<'a>(SeqWriter<'a>);

impl<'a> ObjectWriter<'a> {
    pub fn begin_object(&mut self, name: &str) -> ObjectWriter<'_> {
        self.0.enter_key(Some(name));
        ObjectWriter(SeqWriter::begin(self.0.w, false))
    }
    pub fn begin_array(&mut self, name: &str) -> ArrayWriter<'_> {
        self.0.enter_key(Some(name));
        ArrayWriter(SeqWriter::begin(self.0.w, true))
    }
    pub fn set_compact(&mut self) -> &mut Self {
        self.0.set_compact();
        self
    }
    pub fn compact(mut self) -> Self {
        self.set_compact();
        self
    }
    pub fn put<WJ: ?Sized + WriteJson>(&mut self, k: &str, val: &WJ) -> &mut Self {
        self.0.put_impl(Some(k), &val as &dyn WriteJson);
        self
    }
    pub fn put_all(&mut self, vals: &[(&str, &dyn WriteJson)]) -> &mut Self {
        for kv in vals {
            self.0.put_impl(Some(kv.0), kv.1);
        }
        self
    }
}

struct SeqWriter<'a> {
    w: &'a mut Writer,
    first: bool,
    is_arr: bool,
    tmp_compact: bool,
}
impl<'a> Drop for SeqWriter<'a> {
    fn drop(&mut self) {
        self.w.indent -= 4;
        if !self.first {
            self.w.nl();
            self.w.put_indent();
        }
        self.w.o.push(if self.is_arr { ']' } else { '}' });
        if self.tmp_compact {
            self.w.pop_compact()
        }
    }
}
impl<'a> SeqWriter<'a> {
    fn begin(w: &'a mut Writer, arr: bool) -> Self {
        w.o.push(if arr { '[' } else { '{' });
        w.indent += 4;
        Self {
            w,
            first: true,
            tmp_compact: false,
            is_arr: arr,
        }
    }
    fn set_compact(&mut self) {
        debug_assert!(!self.tmp_compact && self.first);
        self.tmp_compact = true;
        self.w.push_compact();
    }
    fn enter_key(&mut self, k: Option<&str>) {
        if core::mem::replace(&mut self.first, false) {
            self.w.nl();
        } else {
            self.w.comma_nl();
        }
        self.w.put_indent();
        if let Some(k) = k {
            self.w.put_escaped(k, true);
            self.w.ppush(": ", ":");
        }
    }
    fn put_impl(&mut self, k: Option<&str>, v: &dyn WriteJson) {
        assert_eq!(self.is_arr, k.is_none());
        if !v.should_include() {
            return;
        }
        self.enter_key(k);
        v.write_json(self.w);
    }
}
