/*
Define the core data types that everything else uses.
An enum means "exactly one of these things."
A PIIType value is either a Fodselsnummer or
a Phone or a Person — never two at once, never something unlisted.
 */

 enum DetectionMethod {
     Regex,
     SpacyNER,
     Gazetteer,
     ContextRule,
     Manual,
 }

 struct Span {
     pii_type: PIIType,
     source: DetectionSource,
     value: String,
     start: usize,
     end: usize,
     confidence: f64,
     status: SpanStatus,       // review state
     placeholder: String,      // e.g. "[PERSON_1]"
 }


 enum SpanStatus {
     Pending,   // awaiting review
     Approved,  // user confirmed — redact this
     Rejected,  // user said no — keep original text
 }


 struct RedactionResult {
     original: String,
     spans: Vec<Span>,      // each span knows its status
     metadata: Metadata,
 }
