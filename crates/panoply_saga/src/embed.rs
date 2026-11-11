//! Formulas embedded in other assets: a field of type [`Formula`], written in the asset as a
//! string and compiled as the asset loads.

use std::{
    fmt, io,
    marker::PhantomData,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use bevy::{
    log::error,
    reflect::{
        CreateTypeData, PartialReflect, Reflect, ReflectSerialize, TypePath, TypeRegistration,
        TypeRegistry, Typed, serde::ReflectDeserializerProcessor,
    },
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    ExprType, HostState, NoImports, ScriptModule,
    compiler::{compile_formula, format_error, format_span},
};

/// A compiled formula, or the message its compile error gave.
type Compiled = Result<Arc<ScriptModule>, Arc<str>>;

/// Where a formula and its clones keep their compiled form, set once as its asset loads.
type CompiledCell = Arc<OnceLock<Compiled>>;

/// A formula returning a `T`, embedded in an asset. Its compiled form is shared by every clone.
///
/// Only a loader that deserializes through a [`FormulaQueue`] can read one, so that every
/// formula in an asset compiles as the asset loads.
#[derive(Reflect)]
#[reflect(opaque, Clone, Debug, Serialize, Formula)]
pub struct Formula<T: Typed + TypePath + Send + Sync> {
    source: Arc<str>,
    compiled: CompiledCell,
    result: PhantomData<fn() -> T>,
}

impl<T: Typed + TypePath + Send + Sync> Formula<T> {
    /// A formula that has not been compiled, as for a test, or an edit not yet applied.
    pub fn uncompiled(source: impl Into<Arc<str>>) -> Self {
        Self {
            source: source.into(),
            compiled: Arc::default(),
            result: PhantomData,
        }
    }

    /// The formula as written.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The compiled formula, if it compiled.
    pub fn module(&self) -> Option<&Arc<ScriptModule>> {
        self.compiled.get()?.as_ref().ok()
    }

    /// Why the formula did not compile, located in its asset.
    pub fn error(&self) -> Option<&str> {
        self.compiled
            .get()?
            .as_ref()
            .err()
            .map(|message| &**message)
    }
}

impl<T: Typed + TypePath + Send + Sync> Clone for Formula<T> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            compiled: self.compiled.clone(),
            result: PhantomData,
        }
    }
}

impl<T: Typed + TypePath + Send + Sync> fmt::Debug for Formula<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Formula").field(&self.source).finish()
    }
}

impl<T: Typed + TypePath + Send + Sync> Serialize for Formula<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.source)
    }
}

/// Type data for every [`Formula`] type: what it returns, and how to make one.
#[derive(Clone)]
pub struct ReflectFormula {
    result_type: fn() -> ExprType,
    make: fn(Arc<str>, CompiledCell) -> Box<dyn PartialReflect>,
}

impl<T: Typed + TypePath + Send + Sync> CreateTypeData<Formula<T>> for ReflectFormula {
    fn create_type_data(_input: ()) -> Self {
        Self {
            result_type: || ExprType::from_type_info(T::type_info()),
            make: |source, compiled| {
                Box::new(Formula::<T> {
                    source,
                    compiled,
                    result: PhantomData,
                })
            },
        }
    }
}

/// A formula read but not yet compiled.
struct PendingFormula {
    source: Arc<str>,
    result_type: ExprType,
    /// For a JSON asset, the offset just past the formula's closing quote in the file.
    end: Option<usize>,
    compiled: CompiledCell,
}

/// Reads the [`Formula`]s in an asset as a reflection processor, then compiles them.
#[derive(Default)]
pub struct FormulaQueue {
    json_position: Option<Arc<AtomicUsize>>,
    pending: Vec<PendingFormula>,
}

impl FormulaQueue {
    /// A reader of a JSON asset's bytes, to give `serde_json::Deserializer::from_reader`, that
    /// tells the queue where each formula is, so that a compile error names its line in the file.
    /// A reader lends no strings, so nothing deserialized from it can borrow a `&str`.
    pub fn track_json<'b>(&mut self, bytes: &'b [u8]) -> JsonReader<'b> {
        let position = Arc::new(AtomicUsize::new(0));
        self.json_position = Some(position.clone());
        JsonReader { bytes, position }
    }

    /// Compiles every formula read, against `host`. A formula that fails stays uncompiled, with
    /// its error, which is also logged; the rest of the asset loads. `file` is the asset's bytes,
    /// which locate errors in a JSON file.
    pub async fn compile(self, path: &str, host: &HostState, file: &[u8]) {
        for formula in self.pending {
            let result =
                compile_formula(path, &formula.source, host, &NoImports, formula.result_type).await;
            let compiled = match result {
                Ok(module) => Ok(Arc::new(module)),
                Err(err) => {
                    let location = err.location();
                    let message = formula
                        .end
                        .and_then(|end| locate_in_json(file, end))
                        .map(|(text, literal)| {
                            let raw = &file[literal..];
                            format_span(
                                path,
                                text,
                                literal + raw_offset(raw, location.start()),
                                literal + raw_offset(raw, location.end()),
                                &err,
                            )
                        })
                        .unwrap_or_else(|| format_error(path, &formula.source, &err));
                    error!("{message}");
                    Err(message.into())
                }
            };
            // The cell was made for this formula, so it is empty.
            let _ = formula.compiled.set(compiled);
        }
    }
}

impl ReflectDeserializerProcessor for FormulaQueue {
    fn try_deserialize<'de, D>(
        &mut self,
        registration: &TypeRegistration,
        _registry: &TypeRegistry,
        deserializer: D,
    ) -> Result<Result<Box<dyn PartialReflect>, D>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let Some(formula) = registration.data::<ReflectFormula>() else {
            return Ok(Err(deserializer));
        };
        let source: Arc<str> = String::deserialize(deserializer)?.into();
        let compiled = CompiledCell::default();
        self.pending.push(PendingFormula {
            source: source.clone(),
            result_type: (formula.result_type)(),
            end: self
                .json_position
                .as_ref()
                .map(|position| position.load(Ordering::Relaxed)),
            compiled: compiled.clone(),
        });
        Ok(Ok((formula.make)(source, compiled)))
    }
}

/// Reads a JSON asset's bytes, counting them. `serde_json` reads a reader one byte at a time
/// and reads no further into the file than it has parsed, so after a string the count is just
/// past its closing quote.
pub struct JsonReader<'b> {
    bytes: &'b [u8],
    position: Arc<AtomicUsize>,
}

impl io::Read for JsonReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let start = self.position.load(Ordering::Relaxed);
        let count = buf.len().min(self.bytes.len() - start);
        buf[..count].copy_from_slice(&self.bytes[start..start + count]);
        self.position.store(start + count, Ordering::Relaxed);
        Ok(count)
    }
}

/// The file as text, and the offset of the first byte inside the string literal that ends just
/// before `end`.
fn locate_in_json(file: &[u8], end: usize) -> Option<(&str, usize)> {
    let text = std::str::from_utf8(file).ok()?;
    let close = end.checked_sub(1)?;
    if file.get(close) != Some(&b'"') {
        return None;
    }
    // The opening quote is the nearest one before that no backslash escapes: an odd run of
    // backslashes escapes the quote after it.
    let mut open = close;
    loop {
        open = file[..open].iter().rposition(|&b| b == b'"')?;
        let backslashes = file[..open]
            .iter()
            .rev()
            .take_while(|&&b| b == b'\\')
            .count();
        if backslashes % 2 == 0 {
            return Some((text, open + 1));
        }
    }
}

/// The offset in a JSON string literal's raw text, `raw`, of the byte at `decoded` in its value.
fn raw_offset(raw: &[u8], decoded: usize) -> usize {
    let hex = |at: usize| {
        let digits = std::str::from_utf8(raw.get(at..at + 4)?).ok()?;
        u32::from_str_radix(digits, 16).ok()
    };
    let (mut r, mut d) = (0, 0);
    while r < raw.len() && raw[r] != b'"' && d < decoded {
        if raw[r] != b'\\' {
            r += 1;
            d += 1;
        } else if raw.get(r + 1) != Some(&b'u') {
            r += 2;
            d += 1;
        } else {
            let unit = hex(r + 2).unwrap_or(0);
            if (0xD800..0xDC00).contains(&unit) && raw.get(r + 6..r + 8) == Some(b"\\u") {
                // A surrogate pair: one four-byte character.
                r += 12;
                d += 4;
            } else {
                r += 6;
                d += char::from_u32(unit).map_or(3, char::len_utf8);
            }
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_offsets_step_over_escapes() {
        // The `u` of each `\u` escape is split from its backslash only to keep it in the source.
        let raw = concat!(r#"a\"b\nc\"#, "u00e9d", r"\", "ud83d", r"\", r#"ude00e""#);
        let raw = raw.as_bytes();
        // Decoded: a " b \n c é(2) d 😀(4) e
        assert_eq!(raw_offset(raw, 0), 0);
        assert_eq!(raw_offset(raw, 1), 1);
        assert_eq!(raw_offset(raw, 2), 3);
        assert_eq!(raw_offset(raw, 4), 6);
        assert_eq!(raw_offset(raw, 5), 7);
        assert_eq!(raw_offset(raw, 7), 13);
        assert_eq!(raw_offset(raw, 8), 14);
        assert_eq!(raw_offset(raw, 12), 26);
        // Past the end stops at the closing quote.
        assert_eq!(raw_offset(raw, 99), 27);
    }

    #[test]
    fn the_literal_is_found_from_its_end() {
        let file = br#"{"when": "a \"b\" \\"}"#;
        let end = file.len() - 1;
        let (_, literal) = locate_in_json(file, end).unwrap();
        assert_eq!(literal, 10);
    }
}
