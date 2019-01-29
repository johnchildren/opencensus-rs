use std::collections::btree_map;
use std::collections::{BTreeMap, HashSet};

use lazy_static::lazy_static;
use regex::{Regex, RegexSet};

const KEY_MAX_SIZE: usize = 256;
const VALUE_MAX_SIZE: usize = 256;
const MAX_KEY_VALUE_PAIRS: usize = 32;

const KEY_WITHOUT_VENDOR_FORMAT: &str = r"^[a-z][_0-9a-z-*/]{0,255}$";
const KEY_WITH_VENDOR_FORMAT: &str = r"^[a-z][_0-9a-z-*/]{0,240}@[a-z][_0-9a-z-*/]{0,13}$";
const VALUE_FORMAT: &str = r"^[\x20-\x2b\x2d-\x3c\x3e-\x7e]{0,255}[\x21-\x2b\x2d-\x3c\x3e-\x7e]$";

/// Key is an opaque string up to 256 characters printable. It MUST begin with a lowercase letter,
/// and can only contain lowercase letters a-z, digits 0-9, underscores _, dashes -, asterisks *, and
/// forward slashes /.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Key(String);

// TODO(john|p=1|#errors): implement error handling traits
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum KeyValidationError {
    ExceedsMaxSize,
    DoesNotMatchRegex,
}

impl Key {
    pub fn try_new(key: &str) -> Result<Self, KeyValidationError> {
        lazy_static! {
            static ref KEY_VALIDATION_RE: RegexSet =
                RegexSet::new(&[KEY_WITHOUT_VENDOR_FORMAT, KEY_WITH_VENDOR_FORMAT]).unwrap();
        }
        if key.len() > KEY_MAX_SIZE {
            Err(KeyValidationError::ExceedsMaxSize)
        } else if !KEY_VALIDATION_RE.is_match(&key) {
            Err(KeyValidationError::DoesNotMatchRegex)
        } else {
            Ok(Key(key.to_string()))
        }
    }
}

/// Value is an opaque string up to 256 characters printable ASCII RFC0020 characters (i.e., the
/// range 0x20 to 0x7E) except comma , and =.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Value(String);

// TODO(john|p=1|#errors): implement error handling traits
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ValueValidationError {
    ExceedsMaxSize,
    DoesNotMatchRegex,
}

impl Value {
    pub fn try_new(value: &str) -> Result<Self, ValueValidationError> {
        lazy_static! {
            static ref VALUE_VALIDATION_RE: Regex = Regex::new(VALUE_FORMAT).unwrap();
        }
        if value.len() > VALUE_MAX_SIZE {
            Err(ValueValidationError::ExceedsMaxSize)
        } else if !VALUE_VALIDATION_RE.is_match(&value) {
            Err(ValueValidationError::DoesNotMatchRegex)
        } else {
            Ok(Value(value.to_string()))
        }
    }
}

/// Tracestate represents tracing-system specific context in a list of key-value pairs. Tracestate allows different
/// vendors propagate additional information and inter-operate with their legacy Id formats.
// TODO(john|p=3|#go): diverged from Go by using a BTreeMap instead of a slice.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Tracestate(BTreeMap<Key, Value>);

/// Entry represents one key-value pair in a list of key-value pair of Tracestate.
// TODO(john|p=3|#go): diverged from Go by using newtypes and smart constructors.
pub type Entry = (Key, Value);

// TODO(john|p=1|#errors): implement error handling traits
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Error {
    MaxKeyValuePairsExceeded,
    InvalidEntry { entry: Entry },
    DuplicateKey { duplicate: Key },
}

impl Tracestate {
    pub fn try_new(parent: Option<&Tracestate>, entries: &[Entry]) -> Result<Self, Error> {
        if parent.is_none() && entries.is_empty() {
            return Ok(Tracestate(BTreeMap::new()));
        }

        // TODO(john|p=3|#go): diverged from Go by validating entries
        // when constructing keys and values.

        if let Some(duplicate) = contains_duplciate_key(entries) {
            return Err(Error::DuplicateKey { duplicate });
        }

        let mut tracestate = Tracestate(BTreeMap::new());

        if let Some(parent) = parent {
            tracestate
                .0
                .extend(parent.0.iter().map(|(k, v)| (k.clone(), v.clone())));
        }

        tracestate.add(entries)?;

        Ok(tracestate)
    }

    pub fn entries(&self) -> btree_map::Iter<'_, Key, Value> {
        self.0.iter()
    }

    fn add(&mut self, entries: &[Entry]) -> Result<(), Error> {
        // TODO(john|p=3|#go): we diverged from Go here by checking the union of
        // keys instead of deleting the recomputing max.
        let combined: BTreeMap<Key, Value> = self
            .0
            .iter()
            .chain(entries.iter().map(|(k, v)| (k, v)))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if combined.len() > MAX_KEY_VALUE_PAIRS {
            Err(Error::MaxKeyValuePairsExceeded)
        } else {
            self.0 = combined;
            Ok(())
        }
    }
}

/// Returns the first duplicate key in the slice of entries.
fn contains_duplciate_key(entries: &[Entry]) -> Option<Key> {
    let mut key_set = HashSet::new();
    entries
        .iter()
        .filter_map(|(key, _)| key_set.replace(key.clone()))
        .next()
}

#[cfg(test)]
mod test {
    use super::*;

    impl Tracestate {
        fn get(&self, key: &Key) -> Option<&Value> {
            self.0.get(key)
        }
    }

    #[test]
    fn create_with_no_parent() {
        let key1 = Key::try_new("hello").unwrap();
        let value1 = Value::try_new("world").unwrap();

        let entry = (key1.clone(), value1.clone());

        let tracestate = Tracestate::try_new(None, &[entry]).unwrap();

        assert_eq!(tracestate.get(&key1), Some(&value1));
    }

    #[test]
    fn create_from_parent_with_single_key() {
        let key1 = Key::try_new("hello").unwrap();
        let value1 = Value::try_new("world").unwrap();
        let key2 = Key::try_new("foo").unwrap();
        let value2 = Value::try_new("bar").unwrap();

        let entry1 = (key1.clone(), value1.clone());
        let entry2 = (key2.clone(), value2.clone());

        let parent = Tracestate::try_new(None, &[entry1]).unwrap();
        let tracestate = Tracestate::try_new(Some(&parent), &[entry2]).unwrap();

        assert_eq!(tracestate.get(&key2), Some(&value2));
        assert_eq!(tracestate.entries().next(), Some((&key2, &value2)));
        assert_eq!(tracestate.entries().last(), Some((&key1, &value1)));
    }

    #[test]
    fn create_from_parent_with_double_keys() {
        let key1 = Key::try_new("hello").unwrap();
        let value1 = Value::try_new("world").unwrap();
        let key2 = Key::try_new("foo").unwrap();
        let value2 = Value::try_new("bar").unwrap();
        let key3 = Key::try_new("bar").unwrap();
        let value3 = Value::try_new("baz").unwrap();

        let entry1 = (key1.clone(), value1.clone());
        let entry2 = (key2.clone(), value2.clone());
        let entry3 = (key3.clone(), value3.clone());

        let parent = Tracestate::try_new(None, &[entry2, entry1]).unwrap();
        let tracestate = Tracestate::try_new(Some(&parent), &[entry3]).unwrap();

        assert_eq!(tracestate.get(&key3), Some(&value3));
        assert_eq!(tracestate.entries().next(), Some((&key3, &value3)));
        assert_eq!(tracestate.entries().last(), Some((&key1, &value1)));
    }

    #[test]
    fn create_from_parent_with_existing_key() {
        let key1 = Key::try_new("hello").unwrap();
        let value1 = Value::try_new("world").unwrap();
        let key2 = Key::try_new("foo").unwrap();
        let value2 = Value::try_new("bar").unwrap();
        let key3 = Key::try_new("hello").unwrap();
        let value3 = Value::try_new("baz").unwrap();

        let entry1 = (key1.clone(), value1.clone());
        let entry2 = (key2.clone(), value2.clone());
        let entry3 = (key3.clone(), value3.clone());

        let parent = Tracestate::try_new(None, &[entry2, entry1]).unwrap();
        let tracestate = Tracestate::try_new(Some(&parent), &[entry3]).unwrap();

        assert_eq!(tracestate.get(&key3), Some(&value3));
        // TODO(john|p=1|#go|#spec): Entries come out in a different order.
        assert_eq!(tracestate.entries().next(), Some((&key2, &value2)));
        assert_eq!(tracestate.entries().last(), Some((&key3, &value3)));
        assert_eq!(tracestate.entries().len(), 2);
    }

    //TODO(john|p=5|#testing): Rust's mutability is explicit.
    // fn implicit_immutable_trace_state()

    //TODO(john|p=4|#testing): This would make a really nice prop test.
    #[test]
    fn key_with_valid_char() {
        let all_valid = "abcdefghijklmnopqrstuvwxyz0123456789_-*/";

        let res = Key::try_new(all_valid);

        assert!(res.is_ok())
    }

    #[test]
    fn key_with_invalid_char() {
        let bad_keys = vec!["1ab", "1ab2", "Abc", " abc", "a=b"];

        for key in bad_keys {
            assert_eq!(
                Key::try_new(key),
                Err(KeyValidationError::DoesNotMatchRegex)
            )
        }
    }

    #[test]
    fn empty_key() {
        assert_eq!(Key::try_new(""), Err(KeyValidationError::DoesNotMatchRegex))
    }

    #[test]
    fn value_with_invalid_char() {
        let bad_values = vec!["A=B", "A,B", "AB "];

        for value in bad_values {
            assert_eq!(
                Value::try_new(value),
                Err(ValueValidationError::DoesNotMatchRegex)
            )
        }
    }

    #[test]
    fn empty_value() {
        assert_eq!(
            Value::try_new(""),
            Err(ValueValidationError::DoesNotMatchRegex)
        )
    }

    #[test]
    fn invalid_key_length() {
        let too_long: String = std::iter::repeat('a').take(KEY_MAX_SIZE + 1).collect();
        assert_eq!(
            Key::try_new(&too_long),
            Err(KeyValidationError::ExceedsMaxSize)
        )
    }

    #[test]
    fn invalid_value_length() {
        let too_long: String = std::iter::repeat('a').take(VALUE_MAX_SIZE + 1).collect();
        assert_eq!(
            Value::try_new(&too_long),
            Err(ValueValidationError::ExceedsMaxSize)
        )
    }

    #[test]
    fn create_from_array_with_over_limit_kv_pairs() {
        let keys = (0..=MAX_KEY_VALUE_PAIRS)
            .into_iter()
            .map(|i| Key::try_new(&format!("a{}b", i)))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let value = Value::try_new("world").unwrap();
        let entries: Vec<_> = keys.iter().cloned().map(|k| (k, value.clone())).collect();

        let tracestate = Tracestate::try_new(None, &*entries);
        assert_eq!(tracestate, Err(Error::MaxKeyValuePairsExceeded));
    }

    #[test]
    fn create_from_empty_array() {
        let tracestate = Tracestate::try_new(None, &[]).unwrap();

        assert_eq!(tracestate.entries().len(), 0);
    }

    #[test]
    fn create_from_parent_with_over_limit_kv_pairs() {
        let keys = (0..MAX_KEY_VALUE_PAIRS)
            .into_iter()
            .map(|i| Key::try_new(&format!("a{}b", i)))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let value = Value::try_new("world").unwrap();
        let entries: Vec<_> = keys.iter().cloned().map(|k| (k, value.clone())).collect();

        let parent = Tracestate::try_new(None, &*entries).unwrap();

        let key = Key::try_new(&format!("a{}b", MAX_KEY_VALUE_PAIRS)).unwrap();

        let tracestate = Tracestate::try_new(Some(&parent), &[(key, value.clone())]);

        assert_eq!(tracestate, Err(Error::MaxKeyValuePairsExceeded));
    }

    #[test]
    fn create_from_array_with_duplicate_keys() {
        let key1 = Key::try_new("hello").unwrap();
        let value1 = Value::try_new("world").unwrap();
        let key2 = Key::try_new("foo").unwrap();
        let value2 = Value::try_new("bar").unwrap();
        let key3 = Key::try_new("hello").unwrap();
        let value3 = Value::try_new("baz").unwrap();

        let entry1 = (key1.clone(), value1.clone());
        let entry2 = (key2.clone(), value2.clone());
        let entry3 = (key3.clone(), value3.clone());

        let tracestate = Tracestate::try_new(None, &[entry1, entry2, entry3]);

        assert_eq!(
            tracestate,
            Err(Error::DuplicateKey {
                duplicate: key1.clone()
            })
        );
    }

    //TODO(john|p=5|#go|#spec): No concept of nil slice or variadic arguments.
    //fn entries_with_none()
}
