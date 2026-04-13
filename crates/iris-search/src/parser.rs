//! Query language parser.
//!
//! Hand-rolled parser combinator that turns a search string into a structured
//! query AST. Supports operators like `from:`, `to:`, `subject:`, `has:`,
//! `is:`, `before:`, `after:`, `larger:`, `smaller:`, `filename:`, `account:`,
//! exact phrase matching with quotes, and negation with `-`.
