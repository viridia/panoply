use super::Choice;

/// A weighted choice is a collection of choices, each with a probability of being selected.
/// The input is a floating point number between 0 and 1.
pub struct WeightedChoice<T: Choice> {
    choices: Vec<T>,
    total: f32,
}

impl<T: Choice> WeightedChoice<T> {
    pub fn choice(choices: &[T], selection: f32) -> Option<&T> {
        let total = choices.iter().fold(0., |acc, p| acc + p.probability());
        let mut i = selection * total;
        for entry in choices.iter() {
            let p = entry.probability();
            if i <= p {
                return Some(entry);
            }
            i -= p;
        }
        None
    }

    pub fn new(choices: Vec<T>) -> WeightedChoice<T> {
        let total = choices.iter().fold(0., |acc, p| acc + p.probability());
        Self { choices, total }
    }

    pub fn choose(&self, selection: f32) -> Option<&T> {
        let mut i = selection * self.total;
        for entry in self.choices.iter() {
            let p = entry.probability();
            if i <= p {
                return Some(entry);
            }
            i -= p;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choices() {
        #[derive(Debug)]
        struct StringSelection {
            pub probability: f32,
            pub value: String,
        }

        impl Choice for StringSelection {
            fn probability(&self) -> f32 {
                self.probability
            }
        }

        let wr = WeightedChoice::<StringSelection>::new(vec![
            StringSelection {
                value: "one".into(),
                probability: 3.,
            },
            StringSelection {
                value: "two".into(),
                probability: 1.,
            },
        ]);

        assert_eq!(wr.choose(0.).expect("failed").value, "one");
        assert_eq!(wr.choose(0.5).expect("failed").value, "one");
        assert_eq!(wr.choose(0.75).expect("failed").value, "one");
        assert_eq!(wr.choose(0.77).expect("failed").value, "two");
        assert_eq!(wr.choose(1.).expect("failed").value, "two");
    }
}
