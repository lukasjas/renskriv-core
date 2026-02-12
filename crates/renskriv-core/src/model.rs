/*
Define the core data types that everything else uses.
An enum means "exactly one of these things."
A PIIType value is either a Fodselsnummer or
a Phone or a Person — never two at once, never something unlisted.
 */

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PIIType {
    Fodselsnummer,
    Dnummer,
    Phone,
    OrgNumber,
    BankAccount,
    Email,
    PostalCode,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DetectionSource {
    Pattern,
    SpacyNER,
    Gazetteer,
    ContextRule,
    Manual,
}

#[derive(Debug, Clone, Serialize)]
pub struct Span {
    pub pii_type: PIIType,
    pub source: DetectionSource,
    pub value: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f64,
}

/*  pub enum SpanStatus {
    Pending,   // awaiting review
    Approved,  // user confirmed — redact this
    Rejected,  // user said no — keep original text
}


pub struct RedactionResult {
    original: String,
    spans: Vec<Span>,      // each span knows its status
    metadata: Metadata,
} */
