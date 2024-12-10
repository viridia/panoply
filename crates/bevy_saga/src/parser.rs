use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "saga.pest"]
pub struct SagaParser;
