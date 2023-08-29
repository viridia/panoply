use std::{borrow::Cow, fmt};

use winnow::{
    ascii::space0,
    combinator::{alt, opt, preceded, repeat, separated1},
    stream::AsChar,
    token::{one_of, take_while},
    PResult, Parser,
};

/// Represents a predicate which can be used to conditionally style a node.
/// Selectors support a subset of CSS grammar:
///
/// ```
///   &
///   &.name
///   .state > &
///   .state > * > &.name
/// ```
///
/// The last selector in an expression must be the current element ('&').
#[derive(Debug, PartialEq, Clone)]
pub enum Selector<'a> {
    /// If we reach this state, it means the match was successful
    Accept,

    /// Match an element with a specific class name.
    Class(Cow<'a, str>, Box<Selector<'a>>),

    /// Reference to the current element.
    Current(Box<Selector<'a>>),

    /// Reference to the parent of this element.
    Parent(Box<Selector<'a>>),

    /// List of alternate choices.
    Either(Vec<Box<Selector<'a>>>),
}

fn amp<'s>(input: &mut &'s str) -> PResult<()> {
    '&'.void().parse_next(input)
}

fn star<'s>(input: &mut &'s str) -> PResult<()> {
    '*'.void().parse_next(input)
}

fn parent<'s>(input: &mut &'s str) -> PResult<()> {
    (space0, '>', space0).void().parse_next(input)
}

fn comma<'s>(input: &mut &'s str) -> PResult<()> {
    (space0, ',', space0).void().parse_next(input)
}

fn class_name<'s>(input: &mut &'s str) -> PResult<&'s str> {
    preceded(
        '.',
        (
            one_of(AsChar::is_alpha),
            take_while(0.., (AsChar::is_alphanum, '-', '_')),
        ),
    )
    .recognize()
    .parse_next(input)
}

fn simple_selector<'s>(input: &mut &'s str) -> PResult<(Option<char>, Vec<&'s str>)> {
    (opt(alt(('*', '&'))), repeat(0.., class_name)).parse_next(input)
}

fn combo_selector<'s, 'a>(input: &mut &'s str) -> PResult<Box<Selector<'a>>> {
    let mut sel = Box::new(Selector::<'a>::Accept);
    let (prefix, classes) = simple_selector.parse_next(input)?;
    for cls in classes {
        sel = Box::new(Selector::<'a>::Class(cls[1..].to_owned().into(), sel));
    }
    if let Some(ch) = prefix {
        if ch == '&' {
            sel = Box::new(Selector::<'a>::Current(sel));
        }
    }
    Ok(sel)
}

impl<'a> Selector<'a> {
    pub fn parser<'s>(input: &mut &'s str) -> PResult<Box<Selector<'a>>> {
        Self::either.parse_next(input)
    }

    fn either<'s>(input: &mut &'s str) -> PResult<Box<Selector<'a>>> {
        separated1(Self::desc_selector, (space0, ',', space0))
            .map(|mut items: Vec<Box<Selector<'a>>>| {
                if items.len() == 1 {
                    items.pop().unwrap()
                } else {
                    Box::new(Selector::Either(items))
                }
            })
            .parse_next(input)
    }

    fn desc_selector<'s>(input: &mut &'s str) -> PResult<Box<Selector<'a>>> {
        let mut sel = combo_selector.parse_next(input)?;
        while parent.parse_next(input).is_ok() {
            sel = Box::new(Selector::<'a>::Parent(sel));
            let (prefix, classes) = simple_selector.parse_next(input)?;
            for cls in classes {
                sel = Box::new(Selector::<'a>::Class(cls[1..].to_owned().into(), sel));
            }
            if let Some(ch) = prefix {
                if ch == '&' {
                    sel = Box::new(Selector::<'a>::Current(sel));
                }
            }
        }

        Ok(sel)
    }
}

impl<'a> std::str::FromStr for Selector<'a> {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Selector::<'a>::parser
            .parse(input.trim())
            .map(|a| *a)
            .map_err(|e| e.to_string())
    }
}

impl<'a> fmt::Display for Selector<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selector::Accept => Ok(()),
            Selector::Current(prev) => {
                // Because 'current' comes first, reverse order
                let mut str = String::with_capacity(64);
                let mut p = prev.as_ref();
                while let Selector::Class(name, desc) = p {
                    str.insert_str(0, name);
                    str.insert_str(0, ".");
                    p = desc.as_ref()
                }
                str.insert_str(0, "&");
                write!(f, "{}{}", p, str)
            }

            Selector::Class(name, prev) => write!(f, "{}.{}", prev, name),
            Selector::Parent(prev) => match prev.as_ref() {
                Selector::Parent(_) => write!(f, "{}* > ", prev),
                _ => write!(f, "{} > ", prev),
            },
            Selector::Either(items) => {
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    item.fmt(f)?
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_current() {
        assert_eq!(
            "&".parse::<Selector>().unwrap(),
            Selector::Current(Box::new(Selector::Accept))
        );
    }

    #[test]
    fn test_serialize() {
        assert_eq!(
            Selector::Current(Box::new(Selector::Accept)).to_string(),
            "&",
        );
        assert_eq!(
            Selector::Class("x".to_owned().into(), Box::new(Selector::Accept)).to_string(),
            ".x",
        );
        assert_eq!(
            ".foo > &.bar".parse::<Selector>().unwrap().to_string(),
            ".foo > &.bar",
        );
        assert_eq!(
            ".foo > .bar.baz".parse::<Selector>().unwrap().to_string(),
            ".foo > .bar.baz",
        );
        assert_eq!(
            ".foo > * > .bar".parse::<Selector>().unwrap().to_string(),
            ".foo > * > .bar",
        );
        assert_eq!(
            ".foo > &.bar.baz".parse::<Selector>().unwrap().to_string(),
            ".foo > &.bar.baz",
        );
        assert_eq!(
            ".a.b.c > .d.e.f > &.g.h.i"
                .parse::<Selector>()
                .unwrap()
                .to_string(),
            ".a.b.c > .d.e.f > &.g.h.i",
        );
        assert_eq!(
            ".foo, .bar".parse::<Selector>().unwrap().to_string(),
            ".foo, .bar",
        );
    }

    #[test]
    fn test_parse_current_class() {
        assert_eq!(
            "&.foo".parse::<Selector>().unwrap(),
            Selector::Current(Box::new(Selector::Class(
                "foo".to_owned().into(),
                Box::new(Selector::Accept)
            )))
        );
    }

    #[test]
    fn test_parse_class() {
        assert_eq!(
            ".foo".parse::<Selector>().unwrap(),
            Selector::Class("foo".to_owned().into(), Box::new(Selector::Accept))
        );
    }

    #[test]
    fn test_parse_parent() {
        assert_eq!(
            "&.foo > .bar".parse::<Selector>().unwrap(),
            Selector::Class(
                "bar".to_owned().into(),
                Box::new(Selector::Parent(Box::new(Selector::Current(Box::new(
                    Selector::Class("foo".to_owned().into(), Box::new(Selector::Accept))
                )))))
            )
        );

        assert_eq!(
            ".foo > &.bar".parse::<Selector>().unwrap(),
            Selector::Current(Box::new(Selector::Class(
                "bar".to_owned().into(),
                Box::new(Selector::Parent(Box::new(Selector::Class(
                    "foo".to_owned().into(),
                    Box::new(Selector::Accept)
                ))))
            )))
        );
    }

    #[test]
    fn test_either() {
        assert_eq!(
            "&.foo, .bar".parse::<Selector>().unwrap(),
            Selector::Either(vec!(
                Box::new(Selector::Current(Box::new(Selector::Class(
                    "foo".to_owned().into(),
                    Box::new(Selector::Accept)
                )))),
                Box::new(Selector::Class(
                    "bar".to_owned().into(),
                    Box::new(Selector::Accept)
                ))
            ))
        );
    }
}
