use smoljson::write::{Null, Writer};

#[test]
fn test_obj() {
    let mut w = Writer::new(true);
    let mut o = w.object();
    o.put_all(&[("foo", &3), ("a", &"12345 test 123")]);
    o.put("test", &["foo", "bar"] as &[&str]);
    o.put("baz", &Null);
    {
        let _subo = o.begin_object("emptyo");
    }
    {
        let _suba = o.begin_array("emptya");
    }
    {
        let mut subo2 = o.begin_object("ooo");
        subo2.put("a b c", &40.45);
    }
    {
        let mut suba2 = o.begin_array("aaa");
        for i in 1..=3 {
            suba2.put(i * i);
        }
    }
    {
        let mut suba_compact = o.begin_array("aaapak").compact();
        for i in 1..=10 {
            suba_compact.put(i);
        }
    }
    {
        let mut recs = o.begin_array("recs");
        for i in 1..=3 {
            let mut o = recs.begin_object().compact();
            o.put("foo", &i);
            o.put("bar", &(i * i));
            o.put("baz", &[&format!("{}{}{}", i, i, i)[..]] as &[&str]);
            {
                let mut o2 = o.begin_object("quux");
                o2.put("frob", &(i % 2 == 0));
            }
        }
    }
    drop(o);
    assert_eq!(&w.finish(), TJSON0)
}

#[rustfmt::skip]
const TJSON0: &str = r#"{
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
        4,
        9
    ],
    "aaapak": [1,2,3,4,5,6,7,8,9,10],
    "recs": [
        {"foo":1,"bar":1,"baz":["111"],"quux":{"frob":false}},
        {"foo":2,"bar":4,"baz":["222"],"quux":{"frob":true}},
        {"foo":3,"bar":9,"baz":["333"],"quux":{"frob":false}}
    ]
}"#;
