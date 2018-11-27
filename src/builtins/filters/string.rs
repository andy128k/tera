/// Filters operating on string
use std::collections::HashMap;

use regex::{Captures, Regex};
use slug;
use url::percent_encoding::{utf8_percent_encode, EncodeSet};

use unic_segment::GraphemeIndices;

use errors::{Error, Result};
use value::{Value, ValueRef};
use utils;

fn filter_value_error(filter_name: &str, value: &dyn Value, expected_type: &str) -> Error {
    Error::msg(format!(
        "Filter `{}` was called on an incorrect value: got `{:?}` but expected a {}",
        filter_name, value, expected_type
    ))
}

fn filter_arg_error(filter_name: &str, arg_name: &str, value: &dyn Value, expected_type: &str) -> Error {
    Error::msg(format!(
        "Filter `{}` received an incorrect type for arg `{}`: got `{:?}` but expected a {}",
        filter_name, arg_name, value, expected_type
    ))
}

lazy_static! {
    static ref STRIPTAGS_RE: Regex = Regex::new(r"(<!--.*?-->|<[^>]*>)").unwrap();
    static ref WORDS_RE: Regex = Regex::new(r"\b(?P<first>\w)(?P<rest>\w*)\b").unwrap();
}

/// Convert a value to uppercase.
pub fn upper<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("upper", value, "String"))?;
    Ok(ValueRef::owned(s.to_uppercase()))
}

/// Convert a value to lowercase.
pub fn lower<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("lower", value, "String"))?;
    Ok(ValueRef::owned(s.to_lowercase()))
}

/// Strip leading and trailing whitespace.
pub fn trim<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("trim", value, "String"))?;
    Ok(ValueRef::owned(s.trim().to_owned()))
}

/// Truncates a string to the indicated length.
///
/// # Arguments
///
/// * `value`   - The string that needs to be truncated.
/// * `args`    - A set of key/value arguments that can take the following
///   keys.
/// * `length`  - The length at which the string needs to be truncated. If
///   the length is larger than the length of the string, the string is
///   returned untouched. The default value is 255.
/// * `end`     - The ellipsis string to be used if the given string is
///   truncated. The default value is "…".
///
/// # Remarks
///
/// The return value of this function might be longer than `length`: the `end`
/// string is *added* after the truncation occurs.
///
pub fn truncate<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("truncate", value, "String"))?;
    let length = match args.get("length") {
        Some(l) => l.as_uint().ok_or_else(|| filter_arg_error("truncate", "length", &**l, "usize"))? as usize,
        None => 255,
    };
    let end = match args.get("end") {
        Some(l) => l.as_str().ok_or_else(|| filter_arg_error("truncate", "end", &**l, "String"))?,
        None => "…",
    };

    let graphemes = GraphemeIndices::new(s).collect::<Vec<(usize, &str)>>();

    // Nothing to truncate?
    if length >= graphemes.len() {
        return Ok(ValueRef::borrowed(value));
    }

    let result = s[..graphemes[length].0].to_string() + &end;
    Ok(ValueRef::owned(result))
}

/// Gets the number of words in a string.
pub fn wordcount<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("wordcount", value, "String"))?;
    Ok(ValueRef::owned(s.split_whitespace().count()))
}

/// Replaces given `from` substring with `to` string.
pub fn replace<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("replace", value, "String"))?;

    let from = match args.get("from") {
        Some(val) => val.as_str().ok_or_else(|| filter_arg_error("replace", "from", &**val, "String"))?,
        None => return Err(Error::msg("Filter `replace` expected an arg called `from`")),
    };

    let to = match args.get("to") {
        Some(val) => val.as_str().ok_or_else(|| filter_arg_error("replace", "to", &**val, "String"))?,
        None => return Err(Error::msg("Filter `replace` expected an arg called `to`")),
    };

    Ok(ValueRef::owned(s.replace(from, to)))
}

/// First letter of the string is uppercase rest is lowercase
pub fn capitalize<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("capitalize", value, "String"))?;
    let mut chars = s.chars();
    match chars.next() {
        None => Ok(ValueRef::owned("".to_owned())),
        Some(f) => {
            let res = f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase();
            Ok(ValueRef::owned(res))
        }
    }
}

#[derive(Clone)]
struct UrlEncodeSet<'u>(&'u str);

impl<'u> UrlEncodeSet<'u> {
    fn safe_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl<'u> EncodeSet for UrlEncodeSet<'u> {
    fn contains(&self, byte: u8) -> bool {
        if byte >= 48 && byte <= 57 {
            // digit
            false
        } else if byte >= 65 && byte <= 90 {
            // uppercase character
            false
        } else if byte >= 97 && byte <= 122 {
            // lowercase character
            false
        } else if byte == 45 || byte == 46 || byte == 95 {
            // -, . or _
            false
        } else {
            !self.safe_bytes().contains(&byte)
        }
    }
}

/// Percent-encodes reserved URI characters
pub fn urlencode<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("urlencode", value, "String"))?;
    let safe = match args.get("safe") {
        Some(l) => l.as_str().ok_or_else(|| filter_arg_error("urlencode", "safe", &**l, "String"))?,
        None => "/",
    };

    let encoded = utf8_percent_encode(s, UrlEncodeSet(safe)).collect::<String>();
    Ok(ValueRef::owned(encoded))
}

/// Escapes quote characters
pub fn addslashes<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("addslashes", value, "String"))?;
    Ok(ValueRef::owned(s.replace("\\", "\\\\").replace("\"", "\\\"").replace("\'", "\\\'")))
}

/// Transform a string into a slug
pub fn slugify<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("slugify", value, "String"))?;
    Ok(ValueRef::owned(slug::slugify(s)))
}

/// Capitalizes each word in the string
pub fn title<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("title", value, "String"))?;

    Ok(ValueRef::owned(WORDS_RE.replace_all(&s, |caps: &Captures| {
        let first = caps["first"].to_uppercase();
        let rest = caps["rest"].to_lowercase();
        format!("{}{}", first, rest)
    }).to_string()))
}

/// Removes html tags from string
pub fn striptags<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("striptags", value, "String"))?;
    Ok(ValueRef::owned(STRIPTAGS_RE.replace_all(&s, "").to_string()))
}

/// Returns the given text with ampersands, quotes and angle brackets encoded
/// for use in HTML.
pub fn escape_html<'v>(value: &'v dyn Value, _: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("escape_html", value, "String"))?;
    Ok(ValueRef::owned(utils::escape_html(&s)))
}

/// Split the given string by the given pattern.
pub fn split<'v>(value: &'v dyn Value, args: &HashMap<String, Box<dyn Value>>) -> Result<ValueRef<'v>> {
    let s = value.as_str().ok_or_else(|| filter_value_error("split", value, "String"))?;

    let pat = match args.get("pat") {
        Some(pat) => pat.as_str().ok_or_else(|| filter_arg_error("split", "pat", &**pat, "String"))?,
        None => return Err(Error::msg("Filter `split` expected an arg called `pat`")),
    };

    Ok(ValueRef::owned(s.split(pat).map(|p| p.to_owned()).collect::<Vec<String>>()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::to_value;

    use super::*;

    #[test]
    fn test_upper() {
        let result = upper(&to_value("hello").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"HELLO".to_string()));
    }

    #[test]
    fn test_upper_error() {
        let result = upper(&to_value(&50).unwrap(), &HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Filter `upper` was called on an incorrect value: got `50` but expected a String"
        );
    }

    #[test]
    fn test_trim() {
        let result = trim(&to_value("  hello  ").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"hello".to_string()));
    }

    #[test]
    fn test_truncate_smaller_than_length() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("length".to_string(), Box::new(255));
        let result = truncate(&to_value("hello").unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"hello".to_string()));
    }

    #[test]
    fn test_truncate_when_required() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("length".to_string(), Box::new(2));
        let result = truncate(&to_value("日本語").unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"日本…".to_string()));
    }

    #[test]
    fn test_truncate_custom_end() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("length".to_string(), Box::new(2));
        args.insert("end".to_string(), Box::new(""));
        let result = truncate(&to_value("日本語").unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"日本".to_string()));
    }

    #[test]
    fn test_truncate_multichar_grapheme() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("length".to_string(), Box::new(5));
        args.insert("end".to_string(), Box::new("…"));
        let result = truncate(&to_value("👨‍👩‍👧‍👦 family").unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"👨‍👩‍👧‍👦 fam…".to_string()));
    }

    #[test]
    fn test_lower() {
        let result = lower(&to_value("HELLO").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"hello".to_string()));
    }

    #[test]
    fn test_wordcount() {
        let result = wordcount(&to_value("Joel is a slug").unwrap(), &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&4));
    }

    #[test]
    fn test_replace() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("from".to_string(), Box::new("Hello"));
        args.insert("to".to_string(), Box::new("Goodbye"));
        let result = replace(&to_value(&"Hello world!").unwrap(), &args);
        assert!(result.is_ok());
        assert!(result.unwrap().eq(&"Goodbye world!".to_string()));
    }

    #[test]
    fn test_replace_missing_arg() {
        let mut args = HashMap::<String, Box<dyn Value>>::new();
        args.insert("from".to_string(), Box::new("Hello"));
        let result = replace(&to_value(&"Hello world!").unwrap(), &args);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Filter `replace` expected an arg called `to`"
        );
    }

    #[test]
    fn test_capitalize() {
        let tests = vec![("CAPITAL IZE", "Capital ize"), ("capital ize", "Capital ize")];
        for (input, expected) in tests {
            let result = capitalize(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert!(result.unwrap().eq(&expected.to_string()));
        }
    }

    #[test]
    fn test_addslashes() {
        let tests = vec![
            (r#"I'm so happy"#, r#"I\'m so happy"#),
            (r#"Let "me" help you"#, r#"Let \"me\" help you"#),
            (r#"<a>'"#, r#"<a>\'"#),
            (
                r#""double quotes" and \'single quotes\'"#,
                r#"\"double quotes\" and \\\'single quotes\\\'"#,
            ),
            (r#"\ : backslashes too"#, r#"\\ : backslashes too"#),
        ];
        for (input, expected) in tests {
            let result = addslashes(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert!(result.unwrap().eq(&expected.to_string()));
        }
    }

    #[test]
    fn test_slugify() {
        // slug crate already has tests for general slugification so we just
        // check our function works
        let tests =
            vec![(r#"Hello world"#, r#"hello-world"#), (r#"Hello 世界"#, r#"hello-shi-jie"#)];
        for (input, expected) in tests {
            let result = slugify(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert!(result.unwrap().eq(&expected.to_string()));
        }
    }

    #[test]
    fn test_urlencode() {
        let tests = vec![
            (
                r#"https://www.example.org/foo?a=b&c=d"#,
                None,
                r#"https%3A//www.example.org/foo%3Fa%3Db%26c%3Dd"#,
            ),
            (r#"https://www.example.org/"#, Some(""), r#"https%3A%2F%2Fwww.example.org%2F"#),
            (r#"/test&"/me?/"#, None, r#"/test%26%22/me%3F/"#),
            (r#"escape/slash"#, Some(""), r#"escape%2Fslash"#),
        ];
        for (input, safe, expected) in tests {
            let mut args = HashMap::<String, Box<dyn Value>>::new();
            if let Some(safe) = safe {
                args.insert("safe".to_string(), Box::new(safe));
            }
            let result = urlencode(&to_value(input).unwrap(), &args);
            assert!(result.is_ok());
            assert!(result.unwrap().eq(&expected.to_string()));
        }
    }

    #[test]
    fn test_title() {
        let tests = vec![
            ("foo bar", "Foo Bar"),
            ("foo\tbar", "Foo\tBar"),
            ("foo  bar", "Foo  Bar"),
            ("f bar f", "F Bar F"),
            ("foo-bar", "Foo-Bar"),
            ("FOO\tBAR", "Foo\tBar"),
            ("foo (bar)", "Foo (Bar)"),
            ("foo (bar) ", "Foo (Bar) "),
            ("foo {bar}", "Foo {Bar}"),
            ("foo [bar]", "Foo [Bar]"),
            ("foo <bar>", "Foo <Bar>"),
            ("  foo  bar", "  Foo  Bar"),
            ("\tfoo\tbar\t", "\tFoo\tBar\t"),
            ("foo bar ", "Foo Bar "),
            ("foo bar\t", "Foo Bar\t"),
        ];
        for (input, expected) in tests {
            let result = title(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert!(result.unwrap().eq(&expected.to_string()));
        }
    }

    #[test]
    fn test_striptags() {
        let tests = vec![
            (r"<b>Joel</b> <button>is</button> a <span>slug</span>", "Joel is a slug"),
            (r#"<p>just a small   \n <a href="x"> example</a> link</p>\n<p>to a webpage</p><!-- <p>and some commented stuff</p> -->"#,
             r#"just a small   \n  example link\nto a webpage"#),
            (r"<p>See: &#39;&eacute; is an apostrophe followed by e acute</p>", r"See: &#39;&eacute; is an apostrophe followed by e acute"),
            (r"<adf>a", "a"),
            (r"</adf>a", "a"),
            (r"<asdf><asdf>e", "e"),
            (r"hi, <f x", "hi, <f x"),
            ("234<235, right?", "234<235, right?"),
            ("a4<a5 right?", "a4<a5 right?"),
            ("b7>b2!", "b7>b2!"),
            ("</fe", "</fe"),
            ("<x>b<y>", "b"),
            (r#"a<p a >b</p>c"#, "abc"),
            (r#"d<a:b c:d>e</p>f"#, "def"),
            (r#"<strong>foo</strong><a href="http://example.com">bar</a>"#, "foobar"),
        ];
        for (input, expected) in tests {
            let result = striptags(&to_value(input).unwrap(), &HashMap::new());
            assert!(result.is_ok());
            assert!(result.unwrap().eq(&expected));
        }
    }

    #[test]
    fn test_split() {
        let tests: Vec<(_, _, &[&str])> =
            vec![("a/b/cde", "/", &["a", "b", "cde"]), ("hello, world", ", ", &["hello", "world"])];
        for (input, pat, expected) in tests {
            let mut args = HashMap::<String, Box<dyn Value>>::new();
            args.insert("pat".to_string(), Box::new(pat));
            let result = split(&to_value(input).unwrap(), &args).unwrap();
            assert!(result.is_array());
            assert_eq!(result.len().unwrap(), expected.len());
            for index in 0..expected.len() {
                assert!(result.get(index).unwrap().eq(&expected[index]));
            }
        }
    }
}
