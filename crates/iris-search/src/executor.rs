//! Search execution engine.
//!
//! Takes a parsed query AST and translates it into a SQL query that combines
//! indexed column filters with FTS5 MATCH expressions, returning results
//! ranked by BM25 score with a recency boost.
