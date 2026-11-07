//! JSON as the editor writes assets, nav points now and exemplars later, in the form faery's
//! `JSON.stringify` wrote the authored files: serde_json's pretty printing, two-space indented,
//! except that an array holding only numbers stays on one line (`"pos": [41.6, 0, 41.5]`) and a
//! float with no fraction is written as an integer. Saving a file then changes only what was
//! edited. The file ends with a newline.

use std::io;

use serde::Serialize;
use serde_json::ser::{Formatter, PrettyFormatter, Serializer};

/// `value` as the editor writes it to an asset file.
pub fn to_asset_json<T: Serialize + ?Sized>(value: &T) -> serde_json::Result<String> {
    let mut bytes = Vec::new();
    let mut serializer = Serializer::with_formatter(&mut bytes, AssetFormatter::default());
    value.serialize(&mut serializer)?;
    // serde_json writes only UTF-8.
    let pretty = String::from_utf8(bytes).expect("serde_json wrote invalid UTF-8");
    let mut json = inline_number_arrays(&pretty);
    json.push('\n');
    Ok(json)
}

/// Pretty printing, with whole floats written as integers: `0` rather than `0.0`. Typed reads
/// take an integer for a float. Beyond 2^53 a float is written as it is, since not every integer
/// there is exact.
#[derive(Default)]
struct AssetFormatter(PrettyFormatter<'static>);

/// Largest magnitude at which every integer is exact in an `f64`.
const EXACT_INTEGERS: f64 = 9_007_199_254_740_992.0;

/// `value` as an integer, if it is a whole number small enough to write as one. `-0.0` gives 0.
fn as_integer(value: f64) -> Option<i64> {
    (value.is_finite() && value.fract() == 0.0 && value.abs() <= EXACT_INTEGERS)
        .then_some(value as i64)
}

impl Formatter for AssetFormatter {
    fn write_f32<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f32) -> io::Result<()> {
        match as_integer(value.into()) {
            Some(integer) => self.write_i64(writer, integer),
            None => self.0.write_f32(writer, value),
        }
    }

    fn write_f64<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        match as_integer(value) {
            Some(integer) => self.write_i64(writer, integer),
            None => self.0.write_f64(writer, value),
        }
    }

    // The rest is `PrettyFormatter`'s own: the methods it overrides, delegated.

    fn begin_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_array(writer)
    }

    fn end_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_array(writer)
    }

    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.0.begin_array_value(writer, first)
    }

    fn end_array_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_array_value(writer)
    }

    fn begin_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_object(writer)
    }

    fn end_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_object(writer)
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.0.begin_object_key(writer, first)
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_object_value(writer)
    }

    fn end_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_object_value(writer)
    }
}

/// Joins each pretty-printed array of numbers onto one line. Pretty printing breaks every
/// non-empty array across lines, and a string never holds a raw newline, so an array is one to
/// join when the text between its brackets spans lines and holds only numbers, commas and
/// whitespace.
fn inline_number_arrays(pretty: &str) -> String {
    let mut out = String::with_capacity(pretty.len());
    let mut rest = pretty;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..=open]);
        rest = &rest[open + 1..];
        let Some(close) = rest.find(']') else {
            break;
        };
        let inner = &rest[..close];
        let numbers_only = inner
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace() || "-+.eE,".contains(c));
        if inner.contains('\n') && numbers_only && inner.contains(|c: char| c.is_ascii_digit()) {
            let items: Vec<&str> = inner.split(',').map(str::trim).collect();
            out.push_str(&items.join(", "));
            out.push(']');
            rest = &rest[close + 1..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Point {
        name: &'static str,
        pos: [f32; 3],
        scale: f64,
        tags: Vec<&'static str>,
        empty: Vec<f32>,
    }

    #[test]
    fn number_arrays_stay_on_one_line_and_whole_floats_are_integers() {
        let points = [Point {
            name: "warehouse [1,2]",
            pos: [41.598, -0.0, -1.5e-7],
            scale: 3.0,
            tags: vec!["a", "b"],
            empty: vec![],
        }];
        let expected = r#"[
  {
    "name": "warehouse [1,2]",
    "pos": [41.598, 0, -1.5e-7],
    "scale": 3,
    "tags": [
      "a",
      "b"
    ],
    "empty": []
  }
]
"#;
        assert_eq!(to_asset_json(&points).unwrap(), expected);
    }
}
