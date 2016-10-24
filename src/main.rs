#![allow(unused_variables)]
#![allow(unused_mut)]

extern crate esbuild;
extern crate openssl;
extern crate postgres;
extern crate serde;
extern crate serde_json;
extern crate env_logger;

use std::env;
use esbuild::common::{case_type_tree, denormalize_tree};
use esbuild::graph::{connect, CachedGraph};
use esbuild::types::{Datamodel, CachingOptions, NodeTree};


fn main() {
    env_logger::init().unwrap();

    let mut datamodel = &Datamodel::new()
        .load_from_dictionary("src/gdcdictionary/gdcdictionary/schemas")
        .unwrap();

    let host = env::var("PG_HOST").unwrap_or("localhost".to_string());
    let database = env::var("PG_DATABASE").unwrap_or("automated_test".to_string());
    let user = env::var("PG_USER").unwrap_or(env::var("USER").unwrap());
    let password = env::var("PG_PASSWORD").unwrap_or("".to_string());

    let connection = &connect(host, database, user, password).unwrap();
    let graph = &CachedGraph::from_postgres(datamodel, connection).unwrap();
    let options = &CachingOptions::new();

    let cases = graph.nodes_labeled("case");
    let case_type_tree = &case_type_tree();

    for case in cases {
        let case_tree = &NodeTree::construct(graph, case_type_tree, case);
        case_tree.print(0);
        let doc = denormalize_tree(options, graph, case_tree);
    }
}
