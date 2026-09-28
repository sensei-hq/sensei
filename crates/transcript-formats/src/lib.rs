//! Readers for the on-disk transcript formats sensei ingests.
//!
//! This crate exists because there were two implementations of the VS Code
//! journal replay — one in the daemon, one in the offline report tool — and they
//! **disagreed**. Of the four critical defects found in that format (#123),
//! three were cases where one copy was right and the other was wrong, and the
//! fourth had to be fixed twice. One implementation, one set of tests.
//!
//! Everything here treats its input as untrusted: these are files on disk that
//! nothing in sensei wrote, and in the report tool's case they are other
//! people's files. A malformed record degrades to "skip this record", never to
//! an unbounded allocation, a wiped state, or a fabricated value.

pub mod journal;
pub mod paths;
