//! Engine layer - Business logic implementations
//!
//! Engines implement the "how to do it" - the actual algorithms and processing logic.
//! Organized by volatility domains:
//! - text_processing: Quote fixing, whitespace trimming, markdown transformation (high volatility, change together)
//! - analysis: Word frequency analysis (different volatility domain)
//! - validation: Quote validation logic (separate volatility domain)
//! - comparison: Diff computation for comparing text sources (separate volatility domain)
//!
//! Following the Righting Software Method, Engines:
//! - Are stateless and reusable
//! - Implement Strategy pattern via traits
//! - Encapsulate volatility in the ACTIVITY/algorithm
//! - Can be shared between multiple Managers

pub mod traits;

pub mod text_processing {
    pub mod quote_fixer;
    pub mod whitespace_trimmer;
    pub mod markdown_transform;
    pub mod word_replacement;
}

pub mod analysis {
    pub mod word_analyzer;
}

pub mod validation {
    pub mod validator;
}

pub mod comparison {
    pub mod differ;
}
