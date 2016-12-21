//! This module handles all of the JSON production for the GDC
//! portal. Currently, the entire postgresql database is cached to
//! memory.  To save space, edge labels are only maintained if we need
//! to distinguish between two different types of edges between a
//! single pair of node types.
//!
//! Currently, the entire batch of JSON documents is produced at once
//! for reasons that follow. There are two topmost denormalization
//! functions that are called, denormalize_cases() and
//! denormalize_projects(). The former produces all of the case, file,
//! and annotations documents. The latter produces the project
//! summaries.
//!
//! The case denormalization takes the case tree from gdcdatamodel
//! and, starting at a case, walks recursively to all possible
//! children.  Each child's properties are added to the case document
//! at the appropriate level depending on the correlation (one to
//! one=singleton, or one to many=list).  The leaf node for most paths
//! from case are files, which have a special denormalization.
//!
//! When a file is gathered from walking the case path, a deep copy is
//! both added to the cases file list returned for later collection.
//! Denormalizing a case produces a list of files and
//! annotations. Each file is upserted into a persisting list of
//! files.  If after denormalizing case 1 who produced file A, the
//! upsert involves adding to A the list if not present.  If we have
//! already gotten file A from another case, it means that the file
//! came from multiple cases and we have to update file A to also
//! reference case 1.
//!
//! In order to make decrease the processing time, there are a lot of
//! caching initiatives.  The paths from cases to files are
//! cached. The set of files using each data type and experimental
//! strategy are cached.  There is also a caching scheme for
//! remembering which nodes are walked through a lot and remembering
//! which neighbors they have with a given label.
//!
//! - Josh (jsmiller@uchicago.edu)

#![allow(dead_code)]

#[macro_use] extern crate cpython;
#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;
extern crate openssl;
extern crate postgres;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate threadpool;
extern crate walkdir;
extern crate yaml_rust;
extern crate crypto;
extern crate env_logger;
extern crate pbr;

#[macro_use]
pub mod macros;
pub mod errors;
pub mod graph;
pub mod node;
pub mod edge;
pub mod types;
pub mod datamodel;
pub mod pythonffi;
