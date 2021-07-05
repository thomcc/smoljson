use smoljson::{json, *};

const TESTJSON: &str = r#"{
    "foo": 3,
    "a": "12345 test 123",
    "test": [
        "foo",
        "bar"
    ],
    "baz": null,
    "emptyo": {},
    "emptya": [],
    "ooo": {
        "a b c": 40.45
    },
    "aaa": [
        1,
        -4,
        9
    ],
    "aaapak": [1,2,3,4,5,6,7,8,9,10],
    "recs": [
        {"foo":1,"bar":1,"baz":["111"],"quux":{"frob":false}},
        {"foo":2,"bar":4,"baz":["222"],"quux":{"frob":true}},
        {"foo":3,"bar":9,"baz":["333"],"quux":{"frob":false}}
    ]
}"#;
#[test]
fn test_parse() {
    let mut t = Reader::new(TESTJSON);
    let have = Value::from_reader(&mut t).unwrap();
    let want = smoljson::json!({
        "foo": 3,
        "a": "12345 test 123",
        "test": ["foo", "bar"],
        "baz": null,
        "emptyo": {},
        "emptya": [],
        "ooo": { "a b c": 40.45 },
        "aaa": [1, -4, 9],
        "aaapak": [1,2,3,4,5,6,7,8,9,10],
        "recs": [
            {"foo":1,"bar":1,"baz":["111"],"quux":{"frob":false}},
            {"foo":2,"bar":4,"baz":["222"],"quux":{"frob":true}},
            {"foo":3,"bar":9,"baz":["333"],"quux":{"frob":false}}
        ]
    });
    assert_eq!(want, have);
    let s = want.to_string(true);
    let vv = s.parse::<Value<'static>>().unwrap();
    assert_eq!(vv, have);
    assert_eq!(vv, want);
    let vv2 = want.to_string(false).parse::<Value<'static>>().unwrap();
    assert_eq!(vv2, have);
    assert_eq!(vv2, want);
    assert_eq!(vv2, vv);
    assert!(vv["recs"][1]["quux"]["frob"].is_bool());
    assert!(vv["recs"]["nope"]["???"][0].is_null());
    assert_eq!(vv["test"][0].as_str(), Some("foo"));
    let mut vn = vv.clone();
    vn["x"]["y"]["z"] = Value::from(3);
    assert_eq!(&vn["x"]["y"]["z"], &Value::from(3));
}

#[test]
fn test_escapes() {
    let s = r#"
        "\r\n\t\u0020\f\b\\\"\/\ud83d\uDE0B"
    "#;

    let vv = s.parse::<Value<'static>>().expect("should parse");
    let s = vv.as_str().expect("should be str");
    assert_eq!(s, "\r\n\t\u{20}\x0c\x08\\\"/😋", "{:?}", s.as_bytes());
}

#[test]
fn test_escape_replacement() {
    let cases = &[
        (r#""\ud83d""#, "\u{FFFD}"),
        (r#""\ude0b""#, "\u{FFFD}"),
        (r#""\ud83d""#, "\u{FFFD}"),
        (r#""\ud83d1""#, "\u{FFFD}1"),
        (r#""\ude0b\ud83d""#, "\u{FFFD}\u{FFFD}"),
        (r#""\ud83d\ud83d""#, "\u{FFFD}\u{FFFD}"),
        (r#""\ude0b\ude0b""#, "\u{FFFD}\u{FFFD}"),
        (r#""\ud83d\u0020""#, "\u{FFFD} "),
        (r#""\ude0b\ud83d\ude0b""#, "\u{FFFD}😋"),
        (r#"" \ud83d""#, " \u{FFFD}"),
        (r#"" \ude0b""#, " \u{FFFD}"),
        (r#"" \ud83d""#, " \u{FFFD}"),
        (r#"" \ud83d1""#, " \u{FFFD}1"),
        (r#"" \ude0b\ud83d""#, " \u{FFFD}\u{FFFD}"),
        (r#"" \ud83d\ud83d""#, " \u{FFFD}\u{FFFD}"),
        (r#"" \ude0b\ude0b""#, " \u{FFFD}\u{FFFD}"),
        (r#"" \ud83d\u0020""#, " \u{FFFD} "),
        (r#"" \ude0b\ud83d\ude0b""#, " \u{FFFD}😋"),
        (r#"" \ude0b\ud83d\ude0b ""#, " \u{FFFD}😋 "),
        (r#"" \ude0b \ud83d\ude0b ""#, " \u{FFFD} 😋 "),
        (r#"" \ude0b\ud83d \ude0b ""#, " \u{FFFD}\u{FFFD} \u{FFFD} "),
    ];
    for &(json, want) in cases {
        let val = json
            .parse::<Value<'static>>()
            .unwrap_or_else(|e| panic!("should be able to parse {:?}: {:?}", (json, want), e));
        let s = val
            .as_str()
            .unwrap_or_else(|| panic!("should be a str {:?}", (&val, json)));
        assert_eq!(s, want, "test case {:#?}", (json, want, s));
    }
}

#[test]
fn test_escape_surrogates1() {
    for hi in 0xd800..0xdc00 {
        for lo in 0xdc00..0xe000 {
            let utf16: &[u16] = &[hi, lo];
            let json = format!("\"\\u{:04x}\\u{:04x}\"", hi, lo);
            let val = json.parse::<Value<'static>>().unwrap_or_else(|e| {
                panic!("should be able to parse {:#?}: {:?}", (&json, lo, hi), e)
            });

            let expect =
                core::char::decode_utf16(utf16.iter().copied()).collect::<Vec<Result<char, _>>>();

            assert_eq!(expect.len(), 1, "{:#?}", (&json, lo, hi, expect, &val));
            assert!(expect[0].is_ok(), "{:#?}", (&json, lo, hi, expect, &val));

            let utf8 = expect[0].as_ref().unwrap().to_string();
            assert_eq!(
                val.as_str(),
                Some(&utf8[..]),
                "{:#?}",
                (&json, lo, hi, expect, &val, &utf8)
            )
        }
    }
}
#[test]
fn test_escape_surrogates2() {
    let always_lead = 0xd800;
    let always_trail = 0xdc00;
    for u in 0..=0xffff_u16 {
        let cases: &[(String, &[u16])] = &[
            (format!("\\u{:04x}", u), &[u]),
            (format!("\\u{:04x}\\u{:04x}", u, u), &[u, u]),
            (
                format!("\\u{:04x}\\u{:04x}", always_lead, u),
                &[always_lead, u],
            ),
            (
                format!("\\u{:04x}\\u{:04x}", u, always_lead),
                &[u, always_lead],
            ),
            (
                format!("\\u{:04x}\\u{:04x}", always_trail, u),
                &[always_trail, u],
            ),
            (
                format!("\\u{:04x}\\u{:04x}", u, always_trail),
                &[u, always_trail],
            ),
        ];
        for (case, utf16) in cases {
            let expect_chars = core::char::decode_utf16(utf16.iter().cloned())
                .map(|r| r.unwrap_or(core::char::REPLACEMENT_CHARACTER))
                .collect::<Vec<char>>();
            let expect_string: String = expect_chars.into_iter().collect();
            let json = format!("\"{}\"", case);

            let val = json.parse::<Value<'static>>().unwrap_or_else(|e| {
                panic!(
                    "should be able to parse {:#?}: {:?}",
                    (&json, &expect_string, case, utf16),
                    e
                )
            });
            assert_eq!(
                val.as_str(),
                Some(&expect_string[..]),
                "{:#?}",
                (&json, &expect_string, case, utf16, &val)
            );
        }
    }

    //     decode_utf16(v.iter().cloned())
    //        .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
    //     if let Some(c) = core::char::from_u32(u) {
    //         assert!(!(0xd800..0xe000).contains(u), "{:?}", (u, c));

    //     } else {
    //         assert!((0xd800..0xe000).contains(u), "{}", u);
    //     }
    //     for j in json {
    //         let val = json.parse::<Value<'static>>().unwrap_or_else(|e| {
    //             panic!("should be able to parse {:#?}: {:?}", (&json, u), e);
    //         });
    //     }

    //     let expect =
    //         core::char::decode_utf16(utf16.iter().copied()).collect::<Vec<Result<char, _>>>();

    //     assert_eq!(expect.len(), 1, "{:#?}", (&json, lo, hi, expect, &val));
    //     assert!(expect[0].is_ok(), "{:#?}", (&json, lo, hi, expect, &val));

    //     let utf8 = expect[0].as_ref().unwrap().to_string();
    //     assert_eq!(
    //         val.as_str(),
    //         Some(&utf8[..]),
    //         "{:#?}",
    //         (&json, lo, hi, expect, &val, &utf8)
    //     )
    // }
}

const DIALECTS: &[Dialect] = &[
    Dialect::STRICT,
    Dialect::STRICT.comments(true),
    // Dialect::STRICT.trailing_comma(true),
    // Dialect::STRICT.comments(true).trailing_comma(true),
];

#[track_caller]
fn check_valid(s: &str, want: impl Into<Option<ValOwn>>, d: Dialect) {
    let want = want.into();
    let parse_res = Value::from_str_with(s, d);
    let parsed = match parse_res {
        Ok(got) => got,
        Err(e) => panic!(
            "Should be valid: {:?}. Wanted `{:?}`. Error: {:?}. Dialect {:?}",
            s, want, e, d
        ),
    };
    if let Some(want) = want {
        assert_eq!(
            parsed, want,
            "Got unexpected value from {:?}. (under {:?})",
            s, d
        );
    }
}

#[track_caller]
fn check_invalid(s: &str, d: impl Into<Option<Dialect>>) {
    let d: Option<Dialect> = d.into();
    let parse_res = Value::from_str_with(s, d.unwrap_or(Dialect::STRICT));
    if let Ok(got) = parse_res {
        panic!("Should be invalid: {:?}, got `{:?}` under {:?}", s, got, d);
    }
}

#[track_caller]
fn all(s: &str, want: impl Into<Option<ValOwn>>) {
    let want = want.into();
    let inps = &[
        s.to_string(),
        format!("  {}", s),
        format!("{}  ", s),
        format!("  {}  ", s),
    ];
    for s in inps {
        for &d in DIALECTS {
            check_valid(s, want.clone(), d);
        }
    }
}

#[track_caller]
fn none(s: &str) {
    for &d in DIALECTS {
        check_invalid(s, d);
    }
}

#[track_caller]
fn need_comments(s: &str, want: impl Into<Option<ValOwn>> + Clone) {
    let want = want.into();
    for &d in DIALECTS {
        if d.allow_comments {
            check_valid(s, want.clone(), d);
        } else {
            check_invalid(s, d);
        }
    }
}

#[test]
fn test_focused() {
    all("true", json!(true));
    all("false", json!(false));
    all("null", json!(null));
    all(r#""foo""#, json!("foo"));
    all(
        r#""\" \\ / \b \f \n \r \t""#,
        json!("\" \\ / \x08 \x0c \n \r \t"),
    );
    all(r#""\u00DC""#, json!(r"Ü"));
    all(r#"9"#, json!(9));
    all(r#"-9"#, json!(-9));
    all(r#"0.125"#, json!(0.125));
    all(r#"2e3"#, json!(2e3));
    all(r#"2e+3"#, json!(2e+3));
    all(r#"2e-3"#, json!(2e-3));
    all(r#"2.5e3"#, json!(2.5e3));
    all(r#"2.5E+3"#, json!(2.5e+3));
    all(r#"2.5E-3"#, json!(2.5e-3));
    all("{}", json!({}));
    all("[]", json!([]));
    none("1 2");
    none("1, 2");
    none("1/**/2");

    need_comments("/**/5", json!(5));
    need_comments("5/**/", json!(5));
    need_comments("5//", json!(5));
    need_comments("//\n5", json!(5));
    all("[ [],  [ [] ]]", json!([[], [[]]]));
    all(r#"{ "foo": true }"#, json!({ "foo": true }));
    all(
        r#"{ "foo": true, "": 123 }"#,
        json!({ "foo": true, "": 123 }),
    );
    all(
        r#"{ "mado": "homu", "kyubey": false }"#,
        json!({ "mado": "homu", "kyubey": false }),
    );
    all(
        r#"{ "comments": ["/*", "*/", "//"] }"#,
        json!({ "comments": ["/*", "*/", "//"] }),
    );
    need_comments(r#"{ "test": /*hello*/true }"#, json!({ "test": true }));
    need_comments(
        r#"/**/{ "test": // what?
true }/**/"#,
        json!({ "test": true }),
    );
    need_comments(
        r#"{ "test": //
true }"#,
        json!({ "test": true }),
    );
    need_comments("[5/* // */, 6]", json!([5, 6]));
    need_comments("[5//,\n,6]", json!([5, 6]));
    need_comments("[5/*, 7*/,6]", json!([5, 6]));
    need_comments("[/*[]*/]", json!([]));
    need_comments("[//]\n]", json!([]));

    none("tr/**/ue");
    none("tr/*/ue");
}
