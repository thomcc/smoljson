// Based on the similar macro from serde_json

#[macro_export]
macro_rules! json {
    (null) => ($crate::Value::Null);
    (true) => ($crate::Value::Bool(true));
    (false) => ($crate::Value::Bool(false));
    ({}) => ($crate::Value::Object($crate::alloc::collections::BTreeMap::new()));
    ([]) => ($crate::Value::Array($crate::alloc::vec![]));
    ([ $($tt:tt)+ ]) => {
        $crate::__munch_json_array!([] $($tt)+)
    };
    ({ $($tt:tt)+ }) => {
        $crate::Value::Object({
            let mut object = $crate::alloc::collections::BTreeMap::new();
            $crate::__munch_json_object!(object () ($($tt)+) ($($tt)+));
            object
        })
    };
    ($other:expr) => {
        $crate::Value::from($other)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __munch_json_array {
    ([$($elems:expr,)*]) => {
        $crate::Value::Array($crate::alloc::vec![$($elems,)*])
    };
    ([$($elems:expr),*]) => {
        $crate::Value::Array($crate::alloc::vec![$($elems,)*])
    };
    ([$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::__munch_json_array!([$($elems,)* $crate::json!(null)] $($rest)*)
    };
    ([$($elems:expr,)*] true $($rest:tt)*) => {
        $crate::__munch_json_array!([$($elems,)* $crate::json!(true)] $($rest)*)
    };
    ([$($elems:expr,)*] false $($rest:tt)*) => {
        $crate::__munch_json_array!([$($elems,)* $crate::json!(false)] $($rest)*)
    };
    ([$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::__munch_json_array!([$($elems,)* $crate::json!([$($array)*])] $($rest)*)
    };
    ([$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::__munch_json_array!([$($elems,)* $crate::json!({$($map)*})] $($rest)*)
    };
    ([$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::__munch_json_array!([$($elems,)* $crate::json!($next),] $($rest)*)
    };
    ([$($elems:expr,)*] $last:expr) => {
        $crate::__munch_json_array!([$($elems,)* $crate::json!($last)])
    };
    ([$($elems:expr),*] , $($rest:tt)*) => {
        $crate::__munch_json_array!([$($elems,)*] $($rest)*)
    };
    ([$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
        $crate::json_unexpected!($unexpected)
    };
}
#[doc(hidden)]
#[macro_export]
macro_rules! __munch_json_object {
    ($object:ident () () ()) => {};
    ($object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(($($key)+).into(), $value);
        $crate::__munch_json_object!($object () ($($rest)*) ($($rest)*));
    };
    ($object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        $crate::json_unexpected!($unexpected);
    };
    ($object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.insert(($($key)+).into(), $value);
    };
    ($object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::__munch_json_object!($object [$($key)+] ($crate::json!(null)) $($rest)*);
    };
    ($object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::__munch_json_object!($object [$($key)+] ($crate::json!(true)) $($rest)*);
    };
    ($object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::__munch_json_object!($object [$($key)+] ($crate::json!(false)) $($rest)*);
    };
    ($object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::__munch_json_object!($object [$($key)+] ($crate::json!([$($array)*])) $($rest)*);
    };
    ($object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::__munch_json_object!($object [$($key)+] ($crate::json!({$($map)*})) $($rest)*);
    };
    ($object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::__munch_json_object!($object [$($key)+] ($crate::json!($value)) , $($rest)*);
    };
    ($object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::__munch_json_object!($object [$($key)+] ($crate::json!($value)));
    };
    ($object:ident ($($key:tt)+) (:) $copy:tt) => {
        $crate::json!();
    };
    ($object:ident ($($key:tt)+) () $copy:tt) => {
        $crate::json!();
    };
    ($object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        $crate::json_unexpected!($colon);
    };
    ($object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        $crate::json_unexpected!($comma);
    };
    ($object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::__munch_json_object!($object ($key) (: $($rest)*) (: $($rest)*));
    };
    ($object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        $crate::json_expect_expr_comma!($($unexpected)+);
    };
    ($object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::__munch_json_object!($object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! json_expect_expr_comma {
    ($e:expr , $($tt:tt)*) => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! json_unexpected {
    () => {};
}
