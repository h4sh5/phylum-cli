use std::{
    borrow::{Borrow, Cow},
    collections::HashSet,
    hash::Hash,
};

use anyhow::bail;
use indexmap::IndexMap;
use nom::{
    bytes::complete::is_not,
    character::complete::{anychar, char, satisfy},
    combinator::{all_consuming, cut, map, map_res, value},
    multi::{fold_many0, fold_many_m_n, many0_count, separated_list1},
    sequence::{pair, preceded, terminated},
    Finish, Offset,
};

use crate::lockfiles::ParseResult;

use super::*;

/// Parse a Yarn lockfile and extract [`PackageDescriptor`] information.
pub fn parse(input: &str) -> ParseResult {
    let map = match all_consuming(preceded(opt(yarn_line_ending), yarn_map(0)))(input).finish() {
        Ok((_, map)) => map,
        Err(parse_error) => bail!("{}", nom::error::convert_error(input, parse_error)),
    };

    let mut result = Vec::with_capacity(map.len());

    // Lockfiles may contains multiple keys for the same package.
    // Only output unique package descriptors.
    let mut seen = HashSet::with_capacity(map.len());

    for (name_version, details) in map {
        let name_version_str: &str = name_version.borrow();
        let name = match recognize(preceded(opt(char('@')), is_not("@")))(name_version_str).finish()
        {
            Ok((_, name)) => name.to_string(),
            Err(error) => bail!(
                "Unable to parse package specifier {:?}: {}",
                name_version,
                nom::error::convert_error(name_version_str, error),
            ),
        };

        let details = match details {
            Value::Map(details) => details,
            Value::String(literal) => {
                let (line, column) = get_position(input, &literal);
                bail!("Unexpected string at line {line} column {column}: {literal:?}",);
            }
        };

        let version = match details.get_key_value("version") {
            Some((_, Value::String(version))) => version.to_string(),
            Some((key, Value::Map(_))) => {
                let (line, column) = get_position(input, key);
                bail!("Unexpected map for property {key:?} at line {line} column {column}",);
            }
            None => {
                let (line, column) = get_position(input, &name_version);
                bail!("Package {name:?} at line {line} column {column} has no version");
            }
        };

        if seen.insert((name.clone(), version.clone())) {
            result.push(PackageDescriptor {
                name: name,
                version: version,
                package_type: PackageType::Npm,
            });
        }
    }

    Ok(result)
}

/// A string parsed from the lockfile.
///
/// The original `&'a str` is maintained for tracking the position and reporting errors.
#[derive(Clone, Debug, Eq)]
struct YarnString<'a>(&'a str, Cow<'a, str>);

impl<'a> PartialEq<str> for YarnString<'a> {
    fn eq(&self, other: &str) -> bool {
        self.1 == other
    }
}

impl<'a, 'b> PartialEq<YarnString<'b>> for YarnString<'a> {
    fn eq(&self, other: &YarnString<'b>) -> bool {
        self.1 == other.1
    }
}

impl<'a> ToString for YarnString<'a> {
    fn to_string(&self) -> String {
        self.1.clone().into_owned()
    }
}

impl<'a> Borrow<str> for YarnString<'a> {
    fn borrow(&self) -> &str {
        self.1.borrow()
    }
}

impl<'a> Hash for YarnString<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

/// Get the line and column position of a [YarnString].
fn get_position<'a>(input: &'a str, value: &YarnString<'a>) -> (usize, usize) {
    let offset = input.offset(value.0);
    let (line, line_start_offset) = input[..offset]
        .char_indices()
        .filter_map(|(i, c)| if c == '\n' { Some(i) } else { None })
        .enumerate()
        .map(|(number_of_newlines, newline_offset)| (number_of_newlines + 1, newline_offset + 1))
        .last()
        .unwrap_or_default();
    let column = input[line_start_offset..offset].chars().count();
    (line + 1, column + 1)
}

/// A value from a Yarn lockfile.
///
/// All non-map values are converted to strings.
#[derive(Clone, Debug, PartialEq)]
enum Value<'a> {
    String(YarnString<'a>),
    Map(IndexMap<YarnString<'a>, Value<'a>>),
}

impl<'a> From<YarnString<'a>> for Value<'a> {
    fn from(value: YarnString<'a>) -> Self {
        Value::String(value)
    }
}

impl<'a> From<IndexMap<YarnString<'a>, Value<'a>>> for Value<'a> {
    fn from(value: IndexMap<YarnString<'a>, Value<'a>>) -> Self {
        Value::Map(value)
    }
}

/// Parse 1 or more lines of whitespace.
///
/// This function treats `""` as zero characters of whitespace followed by a line ending (eof).
///
/// The returned `str` will be either `""` or the start of a line which contains non-whitespace,
/// non-comment text.
fn yarn_line_ending<'a>(input: &'a str) -> Result<&'a str, ()> {
    context("end of line", |mut input: &'a str| {
        (input, _) = terminated(
            value((), tuple((space0, opt(comment)))),
            alt((eof, line_ending)),
        )(input)?;
        while !input.is_empty() {
            input = match opt(terminated(
                value((), tuple((space0, opt(comment)))),
                alt((eof, line_ending)),
            ))(input)?
            {
                (next, Some(())) => next,
                (_, None) => break,
            }
        }
        Ok((input, ()))
    })(input)
}

/// Parse one or more map key for a single value.
///
/// Yarn allows a value to have multiple keys separated by commas.
fn yarn_map_keys<'a>(input: &'a str) -> Result<&'a str, Vec<YarnString<'a>>> {
    separated_list1(pair(char(','), space0), yarn_string)(input)
}

/// Parse a yarn map.
///
/// Each entry in the map must have exactly `indentation` pairs of spaces, including the first one
/// (ie if `indentation` is not 0, the first character of the input must be a space).
fn yarn_map<'a>(
    indentation: usize,
) -> impl FnMut(&'a str) -> Result<&'a str, IndexMap<YarnString<'a>, Value<'a>>> {
    fold_many0(
        preceded(
            yarn_indent(indentation),
            pair(yarn_map_keys, yarn_map_value(indentation)),
        ),
        IndexMap::new,
        |mut map, (keys, value)| {
            for key in keys {
                map.insert(key, value.clone());
            }
            map
        },
    )
}

/// Parse Yarn indentation.
///
/// Yarn uses pairs of spaces for indentation, so this matches `indentation * 2` spaces.
fn yarn_indent<'a>(indentation: usize) -> impl FnMut(&'a str) -> Result<&'a str, ()> {
    context(
        "indentation",
        fold_many_m_n(indentation, indentation, tag("  "), || (), |_, _| ()),
    )
}

/// Parse a Yarn value (string or map) that follows a line break.
///
/// The input string should begin at the start of the first line (before any indentation).
fn yarn_multiline_value<'a>(
    indentation: usize,
) -> impl FnMut(&'a str) -> Result<&'a str, Value<'a>> {
    alt((
        map(
            delimited(yarn_indent(indentation), yarn_string, yarn_line_ending),
            Value::String,
        ),
        map(yarn_map(indentation), Value::Map),
    ))
}

/// Parse a Yarn map entry value.
///
/// The input string should begin immediately after the map entry key.
fn yarn_map_value<'a>(indentation: usize) -> impl FnMut(&'a str) -> Result<&'a str, Value<'a>> {
    move |input: &'a str| {
        alt((
            preceded(
                char(':'),
                cut(preceded(
                    yarn_line_ending,
                    yarn_multiline_value(indentation + 1),
                )),
            ),
            map(
                delimited(space0, yarn_string, yarn_line_ending),
                Value::String,
            ),
            preceded(yarn_line_ending, yarn_multiline_value(indentation + 1)),
        ))(input)
    }
}

/// Parse and discard a comment.
fn comment<'a>(input: &'a str) -> Result<&'a str, ()> {
    value((), tuple((space0, tag("#"), is_not("\r\n"))))(input)
}

/// Parse a literal into a string.
///
/// Technically, the lockfile syntax allows for non-string types, but we don't care about them.
fn yarn_string<'a>(input: &'a str) -> Result<&'a str, YarnString<'a>> {
    // Quoted strings are JSON strings, so just collect the entire string and pass it to serde_json.
    let quoted_string_chars = many0_count(alt((
        recognize(pair(char('\\'), anychar)),
        recognize(is_not(r#"\""#)),
    )));
    let quoted_string = map_res(
        recognize(preceded(
            char('"'),
            cut(terminated(quoted_string_chars, char('"'))),
        )),
        |s: &'a str| serde_json::from_str(s).map(|v| YarnString(s, Cow::Owned(v))),
    );

    let raw_string = map(
        recognize(preceded(
            satisfy(|c| c.is_alphabetic() || "/.-".contains(c)),
            opt(is_not(": \n\r,")),
        )),
        |l| YarnString(l, Cow::Borrowed(l)),
    );

    context("literal", alt((quoted_string, raw_string)))(input)
}

#[cfg(test)]
mod test {
    use indexmap::indexmap;
    use nom::error::{ErrorKind, VerboseErrorKind};

    use super::*;

    impl<'a> From<&'a str> for YarnString<'a> {
        fn from(raw: &'a str) -> Self {
            // This conversion will break `get_position`, but we don't care for most of these tests.
            YarnString(raw, Cow::Borrowed(raw))
        }
    }

    impl<'a> From<&'a str> for Value<'a> {
        fn from(raw: &'a str) -> Self {
            Value::from(YarnString::from(raw))
        }
    }

    macro_rules! yarn_map {
        ($($key:expr => $value:expr,)+) => { yarn_map!($($key => $value),+) };
        ($($key:expr => $value:expr),*) => {
            indexmap!(
                $(
                    YarnString::from($key) => Value::from($value),
                )*
            )
        };
    }

    #[test]
    fn parse_string() {
        assert_eq!(yarn_string("a"), Ok(("", "a".into())));
        assert_eq!(yarn_string("a "), Ok((" ", "a".into())));
        assert_eq!(yarn_string("a:"), Ok((":", "a".into())));
        assert_eq!(yarn_string("a\n"), Ok(("\n", "a".into())));

        assert_eq!(yarn_string(r#""a""#), Ok(("", "a".into())));
        assert_eq!(yarn_string(r#""a" "#), Ok((" ", "a".into())));
        assert_eq!(yarn_string(r#""a":"#), Ok((":", "a".into())));
        assert_eq!(yarn_string(r#""a: ""#), Ok(("", "a: ".into())));

        assert_eq!(
            yarn_string(""),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    ("", VerboseErrorKind::Nom(ErrorKind::Satisfy)),
                    ("", VerboseErrorKind::Nom(ErrorKind::Alt)),
                    ("", VerboseErrorKind::Context("literal"))
                ]
            }))
        );

        assert_eq!(
            yarn_string(" "),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (" ", VerboseErrorKind::Nom(ErrorKind::Satisfy)),
                    (" ", VerboseErrorKind::Nom(ErrorKind::Alt)),
                    (" ", VerboseErrorKind::Context("literal"))
                ]
            }))
        );

        assert_eq!(
            yarn_string(r#""a"#),
            Err(nom::Err::Failure(VerboseError {
                errors: vec![
                    ("", VerboseErrorKind::Char('"')),
                    (r#""a"#, VerboseErrorKind::Context("literal")),
                ]
            }))
        );
    }

    #[test]
    fn parse_line_ending() {
        assert_eq!(yarn_line_ending(""), Ok(("", ())));
        assert_eq!(yarn_line_ending("\n"), Ok(("", ())));
        assert_eq!(yarn_line_ending("\na"), Ok(("a", ())));
        assert_eq!(yarn_line_ending("#comment\na"), Ok(("a", ())));
        assert_eq!(yarn_line_ending(" \n #comment\n\na\n"), Ok(("a\n", ())));
    }

    #[test]
    fn parse_map() {
        let parse = |input| yarn_map(0)(input);

        assert_eq!(
            parse(r#"a "b""#),
            Ok((
                "",
                yarn_map! {
                    "a" => "b",
                }
            ))
        );
        assert_eq!(
            parse(r#""a" "b""#),
            Ok((
                "",
                yarn_map! {
                    "a" => "b",
                }
            ))
        );

        assert_eq!(
            parse("a:\n  b"),
            Ok((
                "",
                yarn_map! {
                    "a" => "b",
                }
            ))
        );

        assert_eq!(
            parse("a:\n  b \"c\""),
            Ok((
                "",
                yarn_map! {
                    "a" => yarn_map! {
                        "b" => "c",
                    },
                }
            ))
        );
        assert_eq!(
            parse("a:\n  b:\n  c \"d\""),
            Ok((
                "",
                yarn_map! {
                    "a" => yarn_map! {
                        "b" => yarn_map!{},
                        "c" => "d",
                    },
                }
            ))
        );

        assert_eq!(
            parse(r#"a,b "c""#),
            Ok((
                "",
                yarn_map! {
                    "a" => "c",
                    "b" => "c",
                }
            ))
        );
    }

    #[test]
    fn get_position_test() {
        let text = "0123\n5678";
        assert_eq!(get_position(text, &YarnString::from(&text[0..1])), (1, 1));
        assert_eq!(get_position(text, &YarnString::from(&text[1..2])), (1, 2));
        assert_eq!(get_position(text, &YarnString::from(&text[3..4])), (1, 4));
        assert_eq!(get_position(text, &YarnString::from(&text[5..6])), (2, 1));
        assert_eq!(get_position(text, &YarnString::from(&text[8..9])), (2, 4));
    }
}
