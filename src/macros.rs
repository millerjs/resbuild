
/// Update a Doc
macro_rules! update {
    ( $original:expr, $new:expr ) => {
        {
            for (key, value) in $new.iter() {
                $original.insert(key.clone(), value.clone());
            }
            $original
        }
    };
}

/// Set a value in a Doc
macro_rules! setitem {
    ( $original:expr, $key:expr, $val:expr ) => {
        {
            use serde_json::to_value;
            $original.insert($key.clone(), to_value(&$val));
        }
    };
}

/// Create a Doc
macro_rules! doc {
    ( { $( $key:expr ; $val:expr ),* } ) => {
        {
            use serde_json::to_value;
            let mut temp_doc = Doc::new();
            $( temp_doc.insert($key.into(), to_value(&$val)); )*
            temp_doc
        }
    };
}


/// Create a HashMap
macro_rules! hash {
    ( { $( $key:expr ; $val:expr ),* } ) => {
        {
            use std::collections::HashMap;
            let mut temp_hash = Hash::new();
            $( temp_hash.insert($key.into(), &$val); )*
            temp_doc
        }
    };
}
