pub trait Choice {
    fn probability(&self) -> f32;
}

pub struct WeightedRandom<T: Choice> {
    choices: Vec<T>,
    total: f32,
}

impl<T: Choice> WeightedRandom<T> {
    pub fn new(choices: Vec<T>) -> WeightedRandom<T> {
        let total = choices.iter().fold(0., |acc, p| acc + p.probability());
        Self { choices, total }
    }

    pub fn choose(&self, selection: f32) -> Option<&T> {
        let mut i = selection * self.total;
        for entry in self.choices.iter() {
            let p = entry.probability();
            if i <= p {
                return Some(&entry);
            }
            i -= p;
        }
        return None;
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

        let wr = WeightedRandom::<StringSelection>::new(vec![
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
