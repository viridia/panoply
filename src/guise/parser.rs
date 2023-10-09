use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "guise/guise.pest"]
pub struct GuiseParser;

#[cfg(test)]
mod tests {
    use pest::Parser;

    use super::*;

    #[test]
    fn test_empty_file() {
        let result = GuiseParser::parse(Rule::guise, "");
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.len(), 1);
        for asset in r.into_iter() {
            match asset.as_rule() {
                Rule::EOI => {}
                _ => panic!("Should be EOI"),
            }
        }
    }

    #[test]
    fn test_empty_group() {
        let result = GuiseParser::parse(Rule::guise, "alice : { }").unwrap();
        for asset in result.into_iter() {
            match asset.as_rule() {
                Rule::decl => {}
                Rule::EOI => {}
                _ => panic!("Should be EOI"),
            }
        }
    }

    #[test]
    fn test_trailing_comma() {
        let result = GuiseParser::parse(Rule::guise, "alice: { bob: {} }");
        assert!(result.is_ok());

        let result = GuiseParser::parse(Rule::guise, "alice: { bob: {}, }");
        assert!(result.is_ok());

        let result = GuiseParser::parse(Rule::guise, "alice: { bob: {}, linda: {} }");
        assert!(result.is_ok());

        let result = GuiseParser::parse(Rule::guise, "alice: { bob: {}, linda: {}, }");
        assert!(result.is_ok());
    }
}
